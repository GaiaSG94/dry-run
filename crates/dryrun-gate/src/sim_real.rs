//! Adaptateur vers le vrai simulateur `dryrun-sim` (SPEC, section 4).
//!
//! Ce module n'est compilé qu'avec la feature `real-sim`. Il ne simule rien
//! lui-même : il traduit les types du simulateur vers ceux du garde-barrière
//! (montants en chaînes, état du coffre aplati) pour que `Gatekeeper` reste
//! identique quel que soit le simulateur branché.

use std::collections::BTreeMap;
use std::path::Path;

use dryrun_core::types::{Decision, Demande, Intent};
use serde_json::Value;

use crate::amount::Amount;
use crate::sim_types::{Deployed, ExecOutcome, VaultStatus};

/// Simulateur réel : fine enveloppe autour de `dryrun_sim::Simulator`.
pub struct Simulator {
    inner: dryrun_sim::Simulator,
}

impl Simulator {
    /// Démarre la chaîne locale et déploie les contrats de démonstration.
    pub async fn start(wasm_dir: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            inner: dryrun_sim::Simulator::start(wasm_dir).await?,
        })
    }

    /// Déploie tous les contrats de démonstration.
    pub async fn deploy_all(&self) -> anyhow::Result<Deployed> {
        let deployed = self.inner.deploy_all().await?;
        Ok(Deployed {
            contracts: deployed.contracts,
        })
    }

    /// Simule une demande et rend la décision du pare-feu.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Decision> {
        self.inner.simulate(demand, intent).await
    }

    /// Exécute réellement une demande sur la chaîne (clé limitée de l'agent).
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome> {
        let outcome = self.inner.execute(demand).await?;
        Ok(ExecOutcome {
            success: outcome.success,
            tx_hash: outcome
                .refusal
                .clone()
                .or_else(|| outcome.message.clone())
                .unwrap_or_else(|| "exécution transmise".to_string()),
            gas_burnt: outcome.gas_burnt,
            logs: outcome.logs,
        })
    }

    /// État du coffre lu sur la chaîne, aplati pour l'API et la page web.
    pub async fn vault_status(&self) -> anyhow::Result<VaultStatus> {
        let raw = self.inner.vault_status().await?;
        let known = self.inner.known_contracts().await.unwrap_or_default();
        Ok(VaultStatus {
            account_id: self.inner.vault().to_string(),
            balance: amount_field(raw.get("balance")),
            tx_cap: amount_field(raw.get("tx_cap")),
            daily_cap: amount_field(raw.get("daily_cap")),
            spent_24h: amount_field(raw.get("spent_24h")),
            is_frozen: raw.frozen(),
            owner: raw.owner().unwrap_or_default(),
            known_contracts: known.into_iter().collect::<BTreeMap<_, _>>(),
            backend: "dryrun-sim".to_string(),
        })
    }

    /// Contrats connus du coffre.
    pub async fn known_contracts(&self) -> anyhow::Result<BTreeMap<String, String>> {
        Ok(self.inner.known_contracts().await?.into_iter().collect())
    }

    /// Gèle le coffre : propriétaire ou clé de gel (le simulateur choisit).
    pub async fn freeze(&self) -> anyhow::Result<()> {
        self.inner.freeze().await.map(|_| ())
    }

    /// Approuve un contrat (action du propriétaire, 1 yoctoNEAR).
    ///
    /// L'empreinte du code est lue sur la chaîne par le simulateur : c'est
    /// celle qui sera comparée au moment de chaque simulation (règle R6).
    pub async fn approve_contract(&self, account: &str, _code_hash: &str) -> anyhow::Result<()> {
        self.inner.approve_contract(account).await.map(|_| ())
    }
}

/// Lit un montant stocké en chaîne, en nombre, ou absent.
fn amount_field(value: Option<&Value>) -> Amount {
    match value {
        Some(Value::String(text)) => Amount(text.parse::<u128>().unwrap_or(0)),
        Some(Value::Number(number)) => Amount(number.as_u64().unwrap_or(0) as u128),
        _ => Amount(0),
    }
}
