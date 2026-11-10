//! Moteur du garde-barrière (Gatekeeper).
//!
//! Une demande suit toujours le même chemin : simulation → verdict → signature
//! ou refus. Le garde-barrière ne détient que la clé limitée de l'agent ; il ne
//! peut ni changer les plafonds, ni approuver un contrat, ni dégeler le coffre.

use std::path::Path;
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent, RuleHit, Verdict};
use serde::{Deserialize, Serialize};

use crate::backend::{Simulator, VaultStatus};
use crate::cardfmt::{format_json, format_terminal, CardView};
use crate::journal::{Journal, JournalEntry};

/// Résultat d'une proposition soumise au garde-barrière.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    /// Verdict du pare-feu.
    pub verdict: Verdict,
    /// Règles qui ont mordu, avec leur détail chiffré.
    pub rules: Vec<RuleHit>,
    /// Carte des effets au format terminal.
    pub card: String,
    /// Même carte, en structure JSON stable pour la page web.
    pub card_view: CardView,
    /// Vrai si la transaction a réellement été signée puis exécutée.
    pub executed: bool,
    /// Explication en une phrase de la suite donnée.
    pub note: String,
}

/// Le garde-barrière : simulation, jugement, puis signature éventuelle.
pub struct Gatekeeper {
    sim: Arc<Simulator>,
    journal: Arc<Journal>,
}

impl Gatekeeper {
    /// Démarre le simulateur puis le garde-barrière.
    pub async fn start(wasm_dir: &Path) -> anyhow::Result<Self> {
        let sim = Simulator::start(wasm_dir).await?;
        Ok(Self::from_sim(Arc::new(sim)))
    }

    /// Construit un garde-barrière autour d'un simulateur déjà démarré.
    pub fn from_sim(sim: Arc<Simulator>) -> Self {
        Self {
            sim,
            journal: Arc::new(Journal::new()),
        }
    }

    /// Journal des demandes déjà traitées.
    pub fn journal(&self) -> &Arc<Journal> {
        &self.journal
    }

    /// Simulateur sous-jacent (lecture d'état, exécution).
    pub fn simulator(&self) -> &Arc<Simulator> {
        &self.sim
    }

    /// Propose une demande avec son intention déclarée.
    ///
    /// `BLOCK` : refus immédiat, rien n'est signé. `WARN` : la décision revient
    /// à l'appelant (confirmation du propriétaire), rien n'est signé ici.
    /// `ALLOW` : la clé limitée de l'agent signe et la demande est exécutée.
    pub async fn propose(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Outcome> {
        let decision = self.sim.simulate(demand, intent).await?;
        let card = format_terminal(&decision, intent);
        let card_view = format_json(&decision, intent);

        let (executed, note) = match decision.verdict {
            Verdict::Block => (
                false,
                "Refusé : le pare-feu a bloqué la demande avant toute signature.".to_string(),
            ),
            Verdict::Warn => (
                false,
                "Avertissement : le propriétaire doit confirmer avant signature.".to_string(),
            ),
            Verdict::Allow => self.execute_allowed(demand).await,
        };

        self.journal.record(
            demand.clone(),
            intent.clone(),
            &decision,
            card.clone(),
            executed,
        );

        Ok(Outcome {
            verdict: decision.verdict,
            rules: decision.rules,
            card,
            card_view,
            executed,
            note,
        })
    }

    /// Signe et exécute une demande autorisée, et rend (succès, explication).
    async fn execute_allowed(&self, demand: &Demande) -> (bool, String) {
        match self.sim.execute(demand).await {
            Ok(outcome) => {
                let note = format!(
                    "Signé par la clé de l'agent puis exécuté (tx {}).",
                    outcome.tx_hash
                );
                (outcome.success, note)
            }
            Err(err) => (false, format!("Échec de l'exécution : {err}")),
        }
    }

    /// Exécute une demande que le propriétaire a explicitement confirmée
    /// malgré un verdict `WARN`. Rend (succès, explication).
    pub async fn confirm(&self, demand: &Demande) -> anyhow::Result<(bool, String)> {
        Ok(self.execute_allowed(demand).await)
    }

    /// Gèle le coffre : un seul geste, via la clé de gel.
    pub async fn freeze(&self) -> anyhow::Result<()> {
        self.sim.freeze().await
    }

    /// État du coffre lu sur la chaîne.
    pub async fn status(&self) -> anyhow::Result<VaultStatus> {
        self.sim.vault_status().await
    }

    /// Enregistre un contrat connu (action réservée au propriétaire).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        self.sim.approve_contract(account, code_hash).await
    }

    /// Frise complète des demandes traitées.
    pub fn log(&self) -> Vec<JournalEntry> {
        self.journal.entries()
    }
}
