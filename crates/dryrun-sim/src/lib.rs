//! `dryrun-sim` — le simulateur du pare-feu « Dry Run ».
//!
//! Deux bacs à sable `near-sandbox` :
//! - la **chaîne** (`chain`) porte les contrats et les comptes de la démo ;
//! - la **simulation** (`sim`) repart de l'état de la chaîne pour chaque
//!   demande, rejoue la transaction *sous le nom du coffre*, puis relit soldes,
//!   clés, code et reçus pour produire un [`Effects`] honnête.
//!
//! Aucune signature n'est faite avec `near-api` : les transactions sont
//! construites et signées avec `near-primitives` + `near-crypto`, puis envoyées
//! en JSON-RPC brut (`src/rpc.rs`). Moins de dépendances, `AccountId` identique
//! à celui de `near-sandbox`.

pub mod accounts;
pub mod cert;
pub mod chain;
pub mod differential;
pub mod effects;
pub mod outcome;
pub mod rpc;
pub mod sandbox_cfg;
pub mod sim;
pub mod util;
pub mod vault_api;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use serde_json::json;

use dryrun_core::types::{KnownContracts, SignedCert};
use dryrun_core::{Decision, Demande, Effects, Intent, RuleCtx, Verdict};

pub use cert::CertSigner;
pub use chain::{AccountState, Chain, Deployed, TxOutcome};
pub use differential::{differential_hit, DifferentialReport};
pub use effects::SnapshotSet;
pub use outcome::Execution;
pub use rpc::Rpc;
pub use sim::{Replay, SimSandbox};
pub use vault_api::VaultStatus;

/// Gaz maximum d'un appel `execute` (la clé d'agent ne peut rien faire d'autre).
pub const EXECUTE_GAS: u64 = 100_000_000_000_000;

/// Fraîcheur d'un certificat, en blocs (vingt blocs, comme l'instantané).
pub const CERT_FRESHNESS_BLOCKS: u64 = 20;

/// Diviseur du montant pour le second passage de la simulation différentielle.
pub const DIFFERENTIAL_DIVISOR: u128 = 100;

/// Sortie de fonds du coffre observée dans un rapport d'effets.
fn outflow_of(effects: &Effects, vault: &str) -> u128 {
    effects
        .near_delta
        .iter()
        .filter(|delta| delta.account == vault && delta.delta < 0)
        .map(|delta| delta.delta.unsigned_abs())
        .sum()
}

/// Ce que l'exécution réelle a donné.
#[derive(Clone, Debug, Default)]
pub struct ExecOutcome {
    /// La transaction a-t-elle réussi ?
    pub success: bool,
    /// Code de refus on-chain (`ERR_FROZEN`, `ERR_OVER_TX_CAP`, ...).
    pub refusal: Option<String>,
    /// Message d'erreur brut, s'il y en a un.
    pub message: Option<String>,
    pub gas_burnt: u64,
    pub logs: Vec<String>,
}

impl From<Execution> for ExecOutcome {
    /// Traduit une exécution en résultat rapportable au garde-barrière.
    fn from(execution: Execution) -> Self {
        ExecOutcome {
            success: execution.success,
            refusal: execution.refusal(),
            message: execution.error.clone(),
            gas_burnt: execution.gas_burnt,
            logs: execution.logs.clone(),
        }
    }
}

/// La façade du simulateur : une chaîne + un rejeu par demande.
pub struct Simulator {
    chain: Chain,
    vault: String,
    signer: CertSigner,
    nonce: AtomicU64,
}

impl Simulator {
    /// Démarre la chaîne locale et déploie les contrats disponibles.
    ///
    /// Le bac à sable de simulation n'est créé qu'au moment d'une simulation :
    /// un bac à sable neuf par demande garantit qu'aucune trace d'une
    /// simulation hostile ne survit (compromis documenté dans le README).
    pub async fn start(wasm_dir: &Path) -> Result<Self> {
        let simulator = Self::start_undeployed(wasm_dir).await?;
        simulator.deploy_all().await?;
        Ok(simulator)
    }

    /// Démarre la chaîne **sans** déployer : l'appelant garde la main sur le
    /// moment du déploiement et récupère son rapport (utile aux tests et à la démo).
    pub async fn start_undeployed(wasm_dir: &Path) -> Result<Self> {
        Ok(Simulator {
            chain: Chain::start(wasm_dir).await?,
            vault: accounts::VAULT.to_string(),
            signer: CertSigner::new(),
            nonce: AtomicU64::new(1),
        })
    }

    /// Déploie tous les contrats de démonstration (idempotent : un contrat dont
    /// le code est déjà en place n'est pas renvoyé).
    pub async fn deploy_all(&self) -> Result<Deployed> {
        self.chain.deploy_all().await
    }

    /// Nom du compte du coffre.
    pub fn vault(&self) -> &str {
        &self.vault
    }

    /// Adresse RPC de la chaîne (utile aux autres crates et aux tests).
    pub fn rpc_url(&self) -> &str {
        self.chain.rpc_url()
    }

    /// Hauteur du dernier bloc de la chaîne.
    pub async fn height(&self) -> Result<u64> {
        self.chain.height().await
    }

    /// Simule une demande et rend la décision complète.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> Result<Decision> {
        let replay = sim::replay(&self.chain, demand, intent).await?;
        let ctx = self.rule_ctx(demand, intent, &replay.effects).await?;
        Ok(decide(&ctx, &replay.effects))
    }

    /// Rejoue la demande et rend les effets **et** l'issue brute de l'exécution.
    pub async fn replay(&self, demand: &Demande, intent: &Intent) -> Result<Replay> {
        sim::replay(&self.chain, demand, intent).await
    }

    /// Simule et rend seulement les effets (pratique pour les tests et la carte).
    pub async fn effects_of(&self, demand: &Demande, intent: &Intent) -> Result<Effects> {
        Ok(sim::replay(&self.chain, demand, intent).await?.effects)
    }

    /// Exécute réellement une demande sur la chaîne, avec la clé d'agent limitée.
    pub async fn execute(&self, demand: &Demande) -> Result<ExecOutcome> {
        self.execute_with_cert(demand, None).await
    }

    /// Exécute une demande avec son certificat de simulation (niveau V1).
    ///
    /// Sans certificat, le coffre n'exécute rien dès qu'une clé de simulateur
    /// est enregistrée : voler la clé de l'agent ne suffit plus.
    pub async fn execute_with_cert(
        &self,
        demand: &Demande,
        cert: Option<&SignedCert>,
    ) -> Result<ExecOutcome> {
        let candidates = match cert {
            Some(signed) => vault_api::execute_args_certified(demand, signed),
            None => vault_api::execute_args(demand),
        };
        let mut last: Option<ExecOutcome> = None;
        for candidate in candidates {
            let execution = self
                .chain
                .call_with_agent(&self.vault, "execute", &candidate, EXECUTE_GAS)
                .await?;
            let outcome = ExecOutcome::from(execution);
            if outcome.success || outcome.refusal.is_some() {
                return Ok(outcome);
            }
            last = Some(outcome);
        }
        Ok(last.unwrap_or_default())
    }

    /// Clé publique du simulateur (hexadécimal), à enregistrer dans le coffre.
    pub fn sim_public_key(&self) -> String {
        self.signer.public_key_hex()
    }

    /// Le propriétaire enregistre la clé du simulateur sur le coffre (1 yocto).
    pub async fn install_sim_key(&self) -> Result<()> {
        let outcome = self
            .chain
            .call(
                accounts::OWNER,
                &self.vault,
                "set_sim_key",
                &vault_api::set_sim_key_args(&self.sim_public_key()),
                1,
                EXECUTE_GAS,
            )
            .await?;
        if !outcome.success {
            anyhow::bail!(
                "enregistrement de la clé de simulateur refusé : {}",
                outcome.error.clone().unwrap_or_default()
            );
        }
        Ok(())
    }

    /// Signe un certificat pour une demande déjà simulée.
    ///
    /// `post` porte les soldes minimaux promis (contrôle au retour, V1).
    pub async fn sign_cert(
        &self,
        demand: &Demande,
        effects: &Effects,
        post: Vec<(String, u128)>,
    ) -> Result<SignedCert> {
        let height = effects
            .snapshot
            .height
            .max(self.chain.height().await.unwrap_or(0));
        let cert = dryrun_core::types::Cert {
            call_digest: dryrun_core::types::Cert::compute_digest(demand),
            snapshot_height: height,
            expires_height: height + CERT_FRESHNESS_BLOCKS,
            max_out: demand.amount().max(outflow_of(effects, &self.vault)),
            post,
            nonce: self.nonce.fetch_add(1, Ordering::SeqCst),
        };
        Ok(self.signer.sign(cert))
    }

    /// Simule deux fois, au montant réel et au centième, et compare les taux.
    ///
    /// Démasque les contrats à double visage et ceux qui reconnaissent le bac
    /// à sable (règle R12).
    pub async fn simulate_differential(
        &self,
        demand: &Demande,
        intent: &Intent,
    ) -> Result<(Decision, DifferentialReport)> {
        let full = sim::replay(&self.chain, demand, intent).await?;
        let ctx = self.rule_ctx(demand, intent, &full.effects).await?;
        let mut decision = decide(&ctx, &full.effects);

        let small_demand = differential::scaled(demand, DIFFERENTIAL_DIVISOR);
        let small = sim::replay(&self.chain, &small_demand, intent).await?;
        let report = differential::compare(&self.vault, intent, &full.effects, &small.effects);
        if let Some(hit) = differential::differential_hit(&report) {
            decision.rules.push(hit);
            decision.verdict = Verdict::Block;
        }
        Ok((decision, report))
    }

    /// Le propriétaire approuve un contrat au code **actuel** (1 yoctoNEAR).
    ///
    /// Rend l'empreinte approuvée : c'est celle que le coffre comparera ensuite
    /// (règle R6). Le propriétaire est le seul à pouvoir le faire (I4).
    pub async fn approve_contract(&self, account: &str) -> Result<String> {
        let state = self.chain.state_of(account).await?;
        if state.code_hash.is_empty() {
            anyhow::bail!("{account} ne porte aucun contrat à approuver");
        }
        let outcome = self
            .chain
            .call(
                accounts::OWNER,
                &self.vault,
                "approve_contract",
                &json!({"account_id": account, "code_hash": state.code_hash}),
                1,
                EXECUTE_GAS,
            )
            .await?;
        if !outcome.success {
            anyhow::bail!(
                "approbation de {account} refusée : {}",
                outcome.error.clone().unwrap_or_default()
            );
        }
        Ok(state.code_hash)
    }

    /// Gèle le coffre avec la clé de gel (seule chose qu'elle puisse faire, I7).
    pub async fn freeze(&self) -> Result<ExecOutcome> {
        Ok(self.chain.freeze(&self.vault).await?.into())
    }

    /// Contrats connus et leur empreinte, lus directement dans l'état du coffre.
    pub async fn is_known(&self, account: &str) -> Result<bool> {
        Ok(vault_api::known(&self.chain, account).await?.is_some())
    }

    /// État du coffre lu sur la chaîne.
    pub async fn vault_status(&self) -> Result<VaultStatus> {
        vault_api::status(&self.chain).await
    }

    /// Contrats connus du coffre : compte → empreinte approuvée.
    pub async fn known_contracts(&self) -> Result<KnownContracts> {
        let mut known = BTreeMap::new();
        for spec in accounts::CONTRACTS {
            if spec.account == accounts::VAULT {
                continue;
            }
            if let Ok(Some(hash)) = vault_api::known(&self.chain, spec.account).await {
                known.insert(spec.account.to_string(), hash);
            }
        }
        Ok(known)
    }

    /// État complet d'un compte de la chaîne.
    pub async fn state_of(&self, account: &str) -> Result<AccountState> {
        self.chain.state_of(account).await
    }

    /// Appel `view` sur la chaîne.
    pub async fn view_call(
        &self,
        account: &str,
        method: &str,
        args: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.chain.view_call(account, method, args).await
    }

    /// Construit le contexte de jugement à partir de la chaîne et des effets.
    async fn rule_ctx(
        &self,
        demand: &Demande,
        intent: &Intent,
        effects: &Effects,
    ) -> Result<RuleCtx> {
        // Plafonds lus sur la chaîne ; valeurs de la SPEC en repli si le coffre
        // n'est pas encore déployé (2 NEAR par transaction, 10 NEAR par jour).
        let (tx_cap, daily_cap) = match vault_api::caps(&self.chain).await {
            Ok((tx_cap, daily_cap)) if tx_cap > 0 => (tx_cap, daily_cap),
            _ => (2 * 10u128.pow(24), 10 * 10u128.pow(24)),
        };
        let daily_spent = vault_api::spent_24h(&self.chain).await.unwrap_or(0);
        let known = self.known_contracts().await.unwrap_or_default();
        Ok(RuleCtx {
            vault: self.vault.clone(),
            demand: demand.clone(),
            intent: intent.clone(),
            known,
            current_code: self.current_codes(demand, effects).await,
            tx_cap,
            daily_cap,
            daily_spent,
            fee_accounts: accounts::FEE_ACCOUNTS
                .iter()
                .map(|name| name.to_string())
                .collect(),
            ..RuleCtx::default()
        })
    }

    /// Empreintes de code actuelles des comptes qui comptent pour la décision.
    async fn current_codes(&self, demand: &Demande, effects: &Effects) -> KnownContracts {
        let mut codes = BTreeMap::new();
        let mut targets: Vec<String> = accounts::CONTRACTS
            .iter()
            .map(|spec| spec.account.to_string())
            .collect();
        if let Some(target) = demand.target() {
            targets.push(target.to_string());
        }
        for snap in &effects.snapshot.accounts {
            targets.push(snap.id.clone());
        }
        targets.sort();
        targets.dedup();
        for account in targets {
            let Ok(id) = account.parse() else { continue };
            match self.chain.rpc.account(&id).await {
                Ok(Some(info)) if info.has_code() => {
                    codes.insert(account, info.code_hash);
                }
                _ => {}
            }
        }
        codes
    }
}

/// Applique les règles pures de `dryrun-core` aux effets observés.
///
/// Point d'entrée unique du jugement : le simulateur ne juge jamais lui-même.
pub fn decide(ctx: &RuleCtx, effects: &Effects) -> Decision {
    dryrun_core::judge(ctx, effects)
}

/// Verdict en une ligne, pour les journaux et la CLI.
pub fn verdict_line(decision: &Decision) -> String {
    let rules: Vec<String> = decision.rule_ids();
    match decision.verdict {
        Verdict::Allow => "ALLOW".to_string(),
        other if rules.is_empty() => other.as_str().to_string(),
        other => format!("{} ({})", other.as_str(), rules.join(", ")),
    }
}

/// Résumé JSON d'une décision (journalisation, page web).
pub fn decision_json(decision: &Decision) -> serde_json::Value {
    json!({
        "verdict": decision.verdict.as_str(),
        "rules": decision.rule_ids(),
        "effects": decision.effects,
    })
}
