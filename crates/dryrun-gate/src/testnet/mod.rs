//! Mode **testnet** : le garde-barrière pilote un coffre `vault` réellement
//! déployé sur le testnet public NEAR.
//!
//! Ce que change ce mode, et rien d'autre :
//!
//! - l'état du coffre (`status()`) est lu **par RPC** sur le réseau public ;
//! - le verdict vient des **mêmes règles** (R1 à R12), mais jugées contre
//!   l'état réel du coffre (plafonds lus sur la chaîne) ;
//! - un verdict `ALLOW` part **sur la chaîne** : l'action est envoyée au
//!   contrat `vault` déployé, signée par la clé du compte, et la réponse
//!   porte l'empreinte de la transaction ;
//! - un verdict `BLOCK` ou `WARN` n'envoie **rien**.
//!
//! La clé privée du compte reste dans le trousseau du système : Dry Run ne la
//! lit jamais, il fait signer par la CLI NEAR (`sign-with-keychain`).
//!
//! Limite assumée : les effets d'un **appel de contrat** ne sont pas rejoués
//! hors chaîne. Un appel vers un contrat inconnu est donc traité comme un
//! premier contact (R5, `WARN`) — rien n'est envoyé sans confirmation du
//! propriétaire. Un transfert, lui, est déterministe : il est jugé exactement.

use std::collections::BTreeMap;

use dryrun_core::judge;
use dryrun_core::rules::RuleCtx;
use dryrun_core::types::{
    Decision, Demande, Effects, Intent, NearDelta, ReceiptInfo, RuleHit, SignedCert, Snapshot,
    Verdict,
};
use serde_json::{json, Value};

pub mod action;
pub mod cli;
pub mod rpc;

use crate::amount::Amount;
use crate::sim_types::{Deployed, DifferentialReport, ExecOutcome, VaultStatus};
use crate::testnet::action::execute_args;
use crate::testnet::cli::NearCli;
use crate::testnet::rpc::RpcClient;

/// Le coffre piloté sur le testnet public.
pub struct Simulator {
    /// Compte qui porte le contrat `vault`.
    vault: String,
    /// Nom du réseau pour la CLI NEAR (`testnet`).
    network: String,
    /// Client RPC (lecture seule).
    rpc: RpcClient,
    /// CLI NEAR (signature et envoi).
    cli: NearCli,
}

/// Vue de `status()`, telle que le contrat la renvoie (montants en chaînes).
#[derive(Debug, serde::Deserialize)]
struct StatusView {
    #[serde(default)]
    owner: String,
    #[serde(default)]
    frozen: bool,
    #[serde(default)]
    tx_cap: String,
    #[serde(default)]
    daily_cap: String,
    #[serde(default)]
    spent_24h: String,
}

impl Simulator {
    /// Prépare le mode testnet : RPC public + CLI NEAR.
    pub async fn start(vault: &str, network: &str, rpc_url: &str) -> anyhow::Result<Self> {
        let cli = NearCli::locate()?;
        let rpc = RpcClient::new(rpc_url)?;
        Ok(Self {
            vault: vault.to_string(),
            network: network.to_string(),
            rpc,
            cli,
        })
    }

    /// Compte du coffre piloté.
    pub fn vault(&self) -> &str {
        &self.vault
    }

    /// Réseau de la CLI.
    pub fn network(&self) -> &str {
        &self.network
    }

    /// Lecture de `status()` par RPC.
    async fn status_view(&self) -> anyhow::Result<StatusView> {
        let raw = self
            .rpc
            .view_call(&self.vault, "status", &json!({}))
            .await?;
        Ok(serde_json::from_value(raw).unwrap_or(StatusView {
            owner: String::new(),
            frozen: false,
            tx_cap: "0".to_string(),
            daily_cap: "0".to_string(),
            spent_24h: "0".to_string(),
        }))
    }

    /// État du coffre, avec les mêmes champs que l'API locale.
    pub async fn vault_status(&self) -> anyhow::Result<VaultStatus> {
        let status = self.status_view().await?;
        let balance = self.rpc.account_amount(&self.vault).await.unwrap_or(0);
        Ok(VaultStatus {
            account_id: self.vault.clone(),
            balance: Amount(balance),
            tx_cap: Amount(yocto_of(&status.tx_cap)),
            daily_cap: Amount(yocto_of(&status.daily_cap)),
            spent_24h: Amount(yocto_of(&status.spent_24h)),
            is_frozen: status.frozen,
            owner: status.owner,
            known_contracts: BTreeMap::new(),
            backend: "testnet".to_string(),
        })
    }

    /// Contexte de jugement, bâti sur l'état **réel** du coffre.
    async fn rule_ctx(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<RuleCtx> {
        let status = self.status_view().await?;
        Ok(RuleCtx {
            vault: self.vault.clone(),
            demand: demand.clone(),
            intent: intent.clone(),
            known: BTreeMap::new(),
            current_code: BTreeMap::new(),
            tx_cap: yocto_of(&status.tx_cap),
            daily_cap: yocto_of(&status.daily_cap),
            daily_spent: yocto_of(&status.spent_24h),
            fee_accounts: vec!["near".to_string()],
            instrument_accounts: Vec::new(),
            ..RuleCtx::default()
        })
    }

    /// Simule une demande et rend la décision du pare-feu.
    ///
    /// Un coffre gelé sur la chaîne bloque d'emblée, comme sur le contrat.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Decision> {
        let ctx = self.rule_ctx(demand, intent).await?;
        if ctx.vault.is_empty() {
            anyhow::bail!("coffre testnet non configuré");
        }
        let frozen = self.status_view().await.map(|s| s.frozen).unwrap_or(false);
        if frozen {
            return Ok(Decision {
                verdict: Verdict::Block,
                rules: vec![frozen_rule()],
                effects: Effects::default(),
            });
        }
        let height = self.rpc.block_height().await.unwrap_or(0);
        let effects = effects_for(demand, &self.vault, height);
        Ok(judge(&ctx, &effects))
    }

    /// Simulation différentielle : hors chaîne, il n'y a pas de rejeu.
    ///
    /// Le rapport rendu est neutre : le contrat à double visage se démasque
    /// par le rejeu au centième, que seule la chaîne permet.
    pub async fn simulate_differential(
        &self,
        demand: &Demande,
        intent: &Intent,
    ) -> anyhow::Result<(Decision, DifferentialReport)> {
        Ok((
            self.simulate(demand, intent).await?,
            DifferentialReport::default(),
        ))
    }

    /// Exécute la demande **sur la chaîne**, signée par la clé du compte.
    ///
    /// Le coffre testnet n'a aucune `sim_key` enregistrée : le certificat de
    /// simulation V1 n'y est pas exigé, il est donc transmis à `null`.
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome> {
        let args = execute_args(demand);
        let call = self.cli.call_function(
            &self.vault,
            "execute",
            &args,
            "0 NEAR",
            &self.vault,
            &self.network,
        )?;
        let success = !call.output.contains("Transaction failed");
        Ok(ExecOutcome {
            success,
            tx_hash: call.tx_hash.clone(),
            gas_burnt: 0,
            logs: vec![explorer_url(&self.network, &call.tx_hash)],
        })
    }

    /// Exécution avec certificat : le testnet n'utilise pas de certificat.
    pub async fn execute_with_cert(
        &self,
        _demand: &Demande,
        _cert: Option<&SignedCert>,
    ) -> anyhow::Result<ExecOutcome> {
        anyhow::bail!("le mode testnet n'utilise pas le certificat de simulation")
    }

    /// Clé publique du simulateur : aucune en mode testnet.
    pub fn sim_public_key(&self) -> String {
        String::new()
    }

    /// Installation de la clé de simulation : sans objet en mode testnet.
    pub async fn install_sim_key(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Certificat de simulation : sans objet en mode testnet.
    pub async fn sign_cert(
        &self,
        _demand: &Demande,
        _effects: &Effects,
        _post: Vec<(String, u128)>,
    ) -> anyhow::Result<SignedCert> {
        anyhow::bail!("le mode testnet n'utilise pas le certificat de simulation")
    }

    /// Contrats connus : le contrat n'expose que leur nombre, pas leur liste.
    pub async fn known_contracts(&self) -> anyhow::Result<BTreeMap<String, String>> {
        Ok(BTreeMap::new())
    }

    /// Gèle le coffre, sur la chaîne.
    ///
    /// Un gel réel est **durable** : le coffre reste gelé au moins une heure
    /// (le dégel exige le propriétaire, 1 yocto et une heure d'attente). Sur
    /// le coffre de démonstration partagé, cela casserait la démo pour le
    /// reste de la journée : le gel est donc refusé, sauf demande explicite
    /// par la variable d'environnement `DRYRUN_ALLOW_TESTNET_FREEZE=1`.
    pub async fn freeze(&self) -> anyhow::Result<()> {
        if std::env::var("DRYRUN_ALLOW_TESTNET_FREEZE").as_deref() != Ok("1") {
            anyhow::bail!(
                "gel d'urgence refusé sur le coffre testnet : le gel est réel et dure au moins \
                 une heure (dégel propriétaire, 1 yocto). Posez \
                 DRYRUN_ALLOW_TESTNET_FREEZE=1 pour le forcer malgré tout."
            );
        }
        self.cli
            .call_function(
                &self.vault,
                "freeze",
                &json!({}),
                "0 NEAR",
                &self.vault,
                &self.network,
            )
            .map(|_| ())
    }

    /// Appel d'une méthode par le propriétaire (geste du propriétaire).
    pub async fn owner_call(
        &self,
        contract: &str,
        method: &str,
        args: &Value,
    ) -> anyhow::Result<()> {
        self.cli
            .call_function(contract, method, args, "0 NEAR", &self.vault, &self.network)
            .map(|_| ())
    }

    /// Approuve un contrat (action du propriétaire, 1 yoctoNEAR).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        let args = json!({ "account_id": account, "code_hash": code_hash });
        self.cli
            .call_function(
                &self.vault,
                "approve_contract",
                &args,
                ONE_YOCTO,
                &self.vault,
                &self.network,
            )
            .map(|_| ())
    }

    /// Le déploiement ne se fait pas depuis le garde-barrière en mode testnet.
    pub async fn deploy_all(&self) -> anyhow::Result<Deployed> {
        anyhow::bail!("le déploiement des contrats de démonstration n'a pas lieu en mode testnet")
    }
}

/// Dépôt exigé par les actions du propriétaire : 1 yoctoNEAR.
const ONE_YOCTO: &str = "0.000000000000000000000001 NEAR";

/// Effets déduits de la demande.
///
/// Un transfert est déterministe : le coffre sort le montant, le destinataire
/// l'encaisse. Un appel de contrat, lui, n'est **pas** rejoué hors chaîne :
/// ses effets restent inconnus, et le pare-feu le traite en premier contact.
fn effects_for(demand: &Demande, vault: &str, height: u64) -> Effects {
    let amount = demand.amount();
    let mut near_delta = Vec::new();
    let mut receipts = Vec::new();
    match demand {
        Demande::Transfer { to, .. } => {
            near_delta.push(NearDelta {
                account: vault.to_string(),
                delta: -(amount as i128),
            });
            near_delta.push(NearDelta {
                account: to.clone(),
                delta: amount as i128,
            });
            receipts.push(ReceiptInfo {
                id: "testnet-transfer".to_string(),
                status: "success".to_string(),
                receiver: to.clone(),
                logs: Vec::new(),
            });
        }
        Demande::FunctionCall { contract, .. } => {
            if amount > 0 {
                near_delta.push(NearDelta {
                    account: vault.to_string(),
                    delta: -(amount as i128),
                });
                near_delta.push(NearDelta {
                    account: contract.clone(),
                    delta: amount as i128,
                });
            }
        }
    }
    Effects {
        snapshot: Snapshot {
            height,
            block_hash: String::new(),
            accounts: vec![dryrun_core::types::AccountSnap {
                id: vault.to_string(),
                code_hash: String::new(),
            }],
        },
        near_delta,
        receipts,
        ..Effects::default()
    }
}

/// Règle synthétique quand le coffre est gelé sur la chaîne.
fn frozen_rule() -> RuleHit {
    RuleHit {
        id: "GEL".to_string(),
        title: "coffre gelé".to_string(),
        detail: "le coffre est gelé sur la chaîne : aucune sortie n'est permise".to_string(),
        severity: Verdict::Block,
    }
}

/// Lit un montant yoctoNEAR rendu en chaîne par le contrat.
fn yocto_of(text: &str) -> u128 {
    text.trim().parse().unwrap_or(0)
}

/// Lien vers l'explorateur du réseau, pour la transaction ejecutée.
fn explorer_url(network: &str, tx_hash: &str) -> String {
    let host = match network {
        "mainnet" => "https://explorer.near.org/transactions/",
        _ => "https://explorer.testnet.near.org/transactions/",
    };
    format!("{host}{tx_hash}")
}
