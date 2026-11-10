//! Simulateur bouchon (`stub`) **local** et en mémoire.
//!
//! Il existe uniquement pour que `dryrun-gate` compile et se teste sans le vrai
//! simulateur `dryrun-sim` (feature `real-sim`). Il respecte exactement la
//! signature décrite à la section 4 du SPEC et n'est **pas** une simulation
//! NEAR : aucun contrat, aucune transaction, aucun reçu réel.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use dryrun_core::judge;
use dryrun_core::rules::RuleCtx;
use dryrun_core::types::{
    AccountSnap, CodeEvent, Decision, Demande, Effects, FtDelta, Intent, KeyEvent, NearDelta,
    NftEvent, ReceiptInfo, RuleHit, SignedCert, Snapshot, Verdict, Yocto,
};

use crate::amount::Amount;
use crate::sim_types::{Caps, Deployed, DifferentialReport, ExecOutcome, VaultStatus};

/// Compte du coffre simulé.
pub const VAULT: &str = "vault.test.near";
/// Propriétaire du coffre.
pub const OWNER: &str = "owner.test.near";

/// Simulateur bouchon : un coffre en mémoire et un jugement par `dryrun_core`.
pub struct Simulator {
    state: RwLock<State>,
}

/// État interne du faux coffre.
struct State {
    balance: Yocto,
    spent_24h: Yocto,
    frozen: bool,
    known: BTreeMap<String, String>,
    height: u64,
}

impl Simulator {
    /// Démarre le faux coffre ; `_wasm_dir` est ignoré.
    pub async fn start(_wasm_dir: &Path) -> anyhow::Result<Self> {
        let mut known = BTreeMap::new();
        known.insert(
            "good-dex.test.near".to_string(),
            "hash_good_dex".to_string(),
        );
        known.insert(
            "test-token.test.near".to_string(),
            "hash_test_token".to_string(),
        );

        Ok(Self {
            state: RwLock::new(State {
                balance: 50 * 10u128.pow(24),
                spent_24h: 0,
                frozen: false,
                known,
                height: 42,
            }),
        })
    }

    /// Déploie les contrats de démonstration (bouchon : noms seulement).
    pub async fn deploy_all(&self) -> anyhow::Result<Deployed> {
        let names = [
            "vault",
            "good-dex",
            "test-token",
            "test-nft",
            "drainer",
            "skimmer",
            "half-step",
            "liar",
            "approver",
            "shapeshifter",
        ];
        let mut contracts = BTreeMap::new();
        for name in names {
            contracts.insert(format!("{name}.test.near"), format!("hash_{name}"));
        }
        Ok(Deployed { contracts })
    }

    /// Simule une demande : fabrique des effets plausibles puis laisse juger.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Decision> {
        let state = read(&self.state)?;
        if state.frozen {
            return Ok(Decision {
                verdict: Verdict::Block,
                rules: vec![frozen_rule()],
                effects: Effects::default(),
            });
        }

        let effects = fake_effects(demand, state.height);
        Ok(judge(&rule_ctx(demand, intent, &state), &effects))
    }

    /// Exécute la demande sur le faux coffre (débit du solde et de la fenêtre 24 h).
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome> {
        let mut state = write(&self.state)?;
        let amount = demand.amount();
        if amount > state.balance {
            anyhow::bail!("solde insuffisant dans le coffre simulé");
        }
        state.balance -= amount;
        state.spent_24h += amount;
        state.height += 1;

        Ok(ExecOutcome {
            success: true,
            tx_hash: format!("stub-tx-{}", state.height),
            gas_burnt: 3_000_000_000_000,
            logs: vec!["EVENT_JSON:{\"event\":\"executed\"}".to_string()],
        })
    }

    /// État du faux coffre.
    /// Simulation différentielle : le bouchon n'en a pas, il rend un rapport neutre.
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

    /// Clé publique factice du bouchon.
    pub fn sim_public_key(&self) -> String {
        "0".repeat(64)
    }

    /// Le bouchon n'a pas de coffre : l'installation est un succès sans effet.
    pub async fn install_sim_key(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Certificat factice : le bouchon ne vérifie rien.
    pub async fn sign_cert(
        &self,
        demand: &Demande,
        _effects: &Effects,
        post: Vec<(String, u128)>,
    ) -> anyhow::Result<SignedCert> {
        Ok(SignedCert {
            cert: dryrun_core::types::Cert {
                call_digest: dryrun_core::types::Cert::compute_digest(demand),
                snapshot_height: 0,
                expires_height: 20,
                max_out: demand.amount(),
                post,
                nonce: 1,
            },
            signature: "0".repeat(128),
            public_key: self.sim_public_key(),
        })
    }

    /// Exécution sans certificat (le bouchon ignore le certificat).
    pub async fn execute_with_cert(
        &self,
        demand: &Demande,
        _cert: Option<&SignedCert>,
    ) -> anyhow::Result<ExecOutcome> {
        self.execute(demand).await
    }

    pub async fn vault_status(&self) -> anyhow::Result<VaultStatus> {
        let state = read(&self.state)?;
        Ok(VaultStatus {
            account_id: VAULT.to_string(),
            balance: Amount(state.balance),
            tx_cap: Amount(TX_CAP),
            daily_cap: Amount(DAILY_CAP),
            spent_24h: Amount(state.spent_24h),
            is_frozen: state.frozen,
            owner: OWNER.to_string(),
            known_contracts: state.known.clone(),
            backend: "stub".to_string(),
        })
    }

    /// Contrats connus du faux coffre.
    pub async fn known_contracts(&self) -> anyhow::Result<BTreeMap<String, String>> {
        Ok(read(&self.state)?.known.clone())
    }

    /// Gèle le faux coffre.
    pub async fn freeze(&self) -> anyhow::Result<()> {
        write(&self.state)?.frozen = true;
        Ok(())
    }

    /// Enregistre un contrat connu (action du propriétaire).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        write(&self.state)?
            .known
            .insert(account.to_string(), code_hash.to_string());
        Ok(())
    }

    /// Plafonds du faux coffre.
    pub async fn caps(&self) -> anyhow::Result<Caps> {
        Ok(Caps {
            tx_cap: Amount(TX_CAP),
            daily_cap: Amount(DAILY_CAP),
        })
    }
}

/// Plafond par transaction du faux coffre : 2 NEAR.
const TX_CAP: Yocto = 2 * 10u128.pow(24);
/// Plafond glissant du faux coffre : 10 NEAR.
const DAILY_CAP: Yocto = 10 * 10u128.pow(24);

/// Lecture de l'état, sans `unwrap` (empoisonnement toléré).
fn read(state: &RwLock<State>) -> anyhow::Result<RwLockReadGuard<'_, State>> {
    state
        .read()
        .map_err(|_| anyhow::anyhow!("verrou de simulation empoisonné"))
}

/// Écriture de l'état, sans `unwrap`.
fn write(state: &RwLock<State>) -> anyhow::Result<RwLockWriteGuard<'_, State>> {
    state
        .write()
        .map_err(|_| anyhow::anyhow!("verrou de simulation empoisonné"))
}

/// Règle synthétique quand le coffre est gelé.
fn frozen_rule() -> RuleHit {
    RuleHit {
        id: "GEL".to_string(),
        title: "coffre gelé".to_string(),
        detail: "le coffre est gelé : aucune sortie n'est permise".to_string(),
        severity: Verdict::Block,
    }
}

/// Contexte de jugement aligné sur les valeurs par défaut de `dryrun_core`.
fn rule_ctx(demand: &Demande, intent: &Intent, state: &State) -> RuleCtx {
    RuleCtx {
        vault: VAULT.to_string(),
        demand: demand.clone(),
        intent: intent.clone(),
        known: state.known.clone(),
        current_code: state.known.clone(),
        tx_cap: TX_CAP,
        daily_cap: DAILY_CAP,
        daily_spent: state.spent_24h,
        fee_accounts: vec!["near".to_string(), "test.near".to_string()],
        gas_threshold: 100 * 10u64.pow(12),
        storage_threshold: 10 * 10u128.pow(24),
        max_touched: 8,
    }
}

/// Effets plausibles d'une demande : le coffre sort, la contrepartie encaisse.
fn fake_effects(demand: &Demande, height: u64) -> Effects {
    let amount = demand.amount();
    let contrepartie = match demand {
        Demande::Transfer { to, .. } => to.clone(),
        Demande::FunctionCall { contract, .. } => contract.clone(),
    };
    Effects {
        snapshot: Snapshot {
            height,
            block_hash: "stub-block-hash".to_string(),
            accounts: vec![AccountSnap {
                id: VAULT.to_string(),
                code_hash: "hash_vault".to_string(),
            }],
        },
        near_delta: vec![
            NearDelta {
                account: VAULT.to_string(),
                delta: -(amount as i128),
            },
            NearDelta {
                account: contrepartie.clone(),
                delta: amount as i128,
            },
        ],
        ft_delta: Vec::<FtDelta>::new(),
        nft: Vec::<NftEvent>::new(),
        keys: Vec::<KeyEvent>::new(),
        code: Vec::<CodeEvent>::new(),
        receipts: vec![ReceiptInfo {
            id: "receipt-stub-1".to_string(),
            status: "success".to_string(),
            receiver: contrepartie,
            logs: Vec::new(),
        }],
        stranded: false,
        gas_burnt: 3_000_000_000_000,
        storage_delta: 0,
    }
}
