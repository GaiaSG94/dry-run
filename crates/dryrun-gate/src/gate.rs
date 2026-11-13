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
    /// Certificat de simulation signé qui accompagne la transaction (V1).
    pub cert: Option<CertView>,
    /// Vrai si la transaction est partie sur une **vraie chaîne** (testnet).
    ///
    /// En mode local, l'exécution se joue dans un bac à sable ou en mémoire :
    /// le dire honnêtement vaut mieux qu'un « oui » trompeur.
    pub on_chain: bool,
    /// Empreinte de la transaction, quand elle est partie sur une vraie chaîne.
    pub tx_hash: Option<String>,
}

/// Vue compacte du certificat de simulation, telle que l'affiche la page web.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CertView {
    /// Condensé SHA-256 de la demande, en hexadécimal.
    pub digest: String,
    /// Signature Ed25519, tronquée pour l'affichage.
    pub signature: String,
    /// Clé publique du simulateur, en hexadécimal.
    pub public_key: String,
    /// Hauteur du bloc simulé.
    pub snapshot_height: u64,
    /// Hauteur au-delà de laquelle le certificat est périmé.
    pub expires_height: u64,
    /// Nonce à usage unique.
    pub nonce: u64,
}

impl From<&dryrun_core::types::SignedCert> for CertView {
    fn from(signed: &dryrun_core::types::SignedCert) -> Self {
        let short = |value: &str, size: usize| -> String {
            if value.len() > size {
                format!("{}…{}", &value[..size], &value[value.len() - 4..])
            } else {
                value.to_string()
            }
        };
        CertView {
            digest: short(&signed.cert.call_digest, 16),
            signature: short(&signed.signature, 16),
            public_key: short(&signed.public_key, 12),
            snapshot_height: signed.cert.snapshot_height,
            expires_height: signed.cert.expires_height,
            nonce: signed.cert.nonce,
        }
    }
}

/// Post-conditions à faire vérifier au retour par le coffre (V1).
///
/// Les minimaux déclarés dans l'intention deviennent des engagements vérifiés
/// on-chain : un écart gèle le coffre et marque le contrat hostile.
pub(crate) fn post_conditions(intent: &Intent) -> Vec<(String, u128)> {
    intent
        .min_in
        .iter()
        .filter_map(|(token, min)| token_account(token).map(|account| (account.to_string(), *min)))
        .collect()
}

/// Compte du contrat qui émet un jeton de la démonstration.
fn token_account(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "TEST" => Some("test-token.test.near"),
        _ => None,
    }
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

    /// Simulation standard du garde-barrière.
    ///
    /// Un appel de contrat avec dépôt est joué **deux fois** (montant réel puis
    /// centième) : la simulation différentielle (R12) fait partie du chemin par
    /// défaut, pas d'un mode à part. Un simple transfert n'a rien à comparer.
    async fn simulate_default(
        &self,
        demand: &Demande,
        intent: &Intent,
    ) -> anyhow::Result<dryrun_core::types::Decision> {
        match demand {
            Demande::FunctionCall { deposit, .. } if *deposit > 0 => {
                let (decision, _report) = self.sim.simulate_differential(demand, intent).await?;
                Ok(decision)
            }
            _ => self.sim.simulate(demand, intent).await,
        }
    }

    /// Propose une demande avec son intention déclarée.
    ///
    /// `BLOCK` : refus immédiat, rien n'est signé. `WARN` : la décision revient
    /// à l'appelant (confirmation du propriétaire), rien n'est signé ici.
    /// `ALLOW` : la clé limitée de l'agent signe et la demande est exécutée,
    /// accompagnée de son **certificat de simulation** (niveau V1).
    pub async fn propose(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Outcome> {
        let decision = self.simulate_default(demand, intent).await?;
        let card = format_terminal(&decision, intent);
        let card_view = format_json(&decision, intent);

        let (executed, note, cert_view, on_chain, tx_hash) = match decision.verdict {
            Verdict::Block => (
                false,
                "Refusé : le pare-feu a bloqué la demande avant toute signature.".to_string(),
                None,
                false,
                None,
            ),
            Verdict::Warn => (
                false,
                "Avertissement : le propriétaire doit confirmer avant signature.".to_string(),
                None,
                false,
                None,
            ),
            Verdict::Allow => self.execute_allowed(demand, intent, &decision).await,
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
            cert: cert_view,
            on_chain,
            tx_hash,
        })
    }

    /// Signe et exécute une demande autorisée.
    ///
    /// Un **certificat de simulation** signé par la clé du simulateur accompagne
    /// la transaction (niveau V1) : le coffre vérifie sa fraîcheur, son nonce et
    /// sa signature avant d'exécuter quoi que ce soit. Sur le testnet, le
    /// coffre n'a aucune `sim_key` enregistrée : l'action part avec
    /// `cert: null`, signée par la clé du compte.
    async fn execute_allowed(
        &self,
        demand: &Demande,
        intent: &Intent,
        decision: &dryrun_core::types::Decision,
    ) -> (bool, String, Option<CertView>, bool, Option<String>) {
        let report = match self
            .sim
            .execute_approved(demand, intent, &decision.effects)
            .await
        {
            Ok(report) => report,
            Err(err) => {
                return (
                    false,
                    format!("Échec de l'exécution : {err}"),
                    None,
                    false,
                    None,
                )
            }
        };
        let cert_view = report.cert.as_ref().map(CertView::from);
        let note = match (report.on_chain, report.tx_hash.as_deref()) {
            (true, Some(hash)) => {
                format!("Signé par la clé du compte puis exécuté sur NEAR testnet (tx {hash}).")
            }
            (true, None) => "Signé par la clé du compte puis exécuté sur NEAR testnet.".to_string(),
            (false, _) => format!(
                "Signé par la clé de l'agent puis exécuté (tx {}).",
                report.outcome.tx_hash
            ),
        };
        (
            report.outcome.success,
            note,
            cert_view,
            report.on_chain,
            report.tx_hash,
        )
    }

    /// Exécute une demande que le propriétaire a explicitement confirmée
    /// malgré un verdict `WARN`. Rend (succès, explication).
    pub async fn confirm(&self, demand: &Demande) -> anyhow::Result<(bool, String)> {
        let intent = Intent::default();
        let decision = self.sim.simulate(demand, &intent).await?;
        let (success, note, _cert, _on_chain, _tx_hash) =
            self.execute_allowed(demand, &intent, &decision).await;
        Ok((success, note))
    }

    /// Enregistre la clé du simulateur sur le coffre (action du propriétaire).
    ///
    /// À partir de là, le coffre refuse toute demande non certifiée : voler la
    /// clé de l'agent ne suffit plus.
    pub async fn install_sim_key(&self) -> anyhow::Result<String> {
        self.sim.install_sim_key().await?;
        Ok(self.sim.sim_public_key())
    }

    /// Appel d'une méthode par le propriétaire (geste du propriétaire).
    pub async fn owner_call(
        &self,
        contract: &str,
        method: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<()> {
        self.sim.owner_call(contract, method, args).await
    }

    /// Clé publique du simulateur (hexadécimal).
    pub fn sim_public_key(&self) -> String {
        self.sim.sim_public_key()
    }

    /// Simulation différentielle (deux montants) : démystifie les contrats à
    /// double visage. Rend la décision et le rapport de divergence.
    pub async fn propose_differential(
        &self,
        demand: &Demande,
        intent: &Intent,
    ) -> anyhow::Result<(Outcome, crate::sim_types::DifferentialReport)> {
        let (decision, report) = self.sim.simulate_differential(demand, intent).await?;
        let card = format_terminal(&decision, intent);
        let card_view = format_json(&decision, intent);
        let (executed, note, cert, on_chain, tx_hash) = match decision.verdict {
            Verdict::Allow => self.execute_allowed(demand, intent, &decision).await,
            Verdict::Warn => (
                false,
                "Avertissement : le propriétaire doit confirmer avant signature.".to_string(),
                None,
                false,
                None,
            ),
            Verdict::Block => (
                false,
                "Refusé : le pare-feu a bloqué la demande avant toute signature.".to_string(),
                None,
                false,
                None,
            ),
        };
        self.journal.record(
            demand.clone(),
            intent.clone(),
            &decision,
            card.clone(),
            executed,
        );
        Ok((
            Outcome {
                verdict: decision.verdict,
                rules: decision.rules,
                card,
                card_view,
                executed,
                note,
                cert,
                on_chain,
                tx_hash,
            },
            report,
        ))
    }

    /// Gèle le coffre : un seul geste, via la clé de gel.
    pub async fn freeze(&self) -> anyhow::Result<()> {
        self.sim.freeze().await
    }

    /// État du coffre lu sur la chaîne.
    pub async fn status(&self) -> anyhow::Result<VaultStatus> {
        self.sim.vault_status().await
    }

    /// Contrats connus du coffre (compte → empreinte approuvée).
    pub async fn known_contracts(
        &self,
    ) -> anyhow::Result<std::collections::BTreeMap<String, String>> {
        self.sim.known_contracts().await
    }

    /// Enregistre un contrat connu (action réservée au propriétaire).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        self.sim.approve_contract(account, code_hash).await
    }

    /// Frise complète des demandes traitées.
    pub fn log(&self) -> Vec<JournalEntry> {
        self.journal.entries()
    }

    /// Appel direct du coffre avec la clé de l'agent, **sans** certificat.
    ///
    /// C'est le scénario du vol de clé : le voleur contourne le garde-barrière.
    /// Au niveau V1 le coffre refuse (`ERR_NO_CERT`) ; sans certificat activé,
    /// seuls les plafonds on-chain le bornent.
    pub async fn execute_without_firewall(
        &self,
        demand: &Demande,
    ) -> anyhow::Result<crate::sim_types::ExecOutcome> {
        self.sim.execute(demand).await
    }
}
