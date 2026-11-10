//! Frise des demandes : chaque entrée garde l'intention, le verdict, les règles
//! et la carte, en mémoire, et s'expose en JSON par `GET /api/log`.

use std::sync::RwLock;

use dryrun_core::types::{Decision, Demande, Intent, RuleHit, Verdict, Yocto};
use serde::{Deserialize, Serialize};

/// Une entrée de la frise, telle que conservée en mémoire.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Rang dans la frise, à partir de 1.
    pub id: u64,
    /// Horodatage Unix, en secondes.
    pub timestamp: u64,
    /// Demande telle que proposée.
    pub demand: Demande,
    /// Intention déclarée.
    pub intent: Intent,
    /// Verdict du pare-feu.
    pub verdict: Verdict,
    /// Règles qui ont mordu.
    pub rules: Vec<RuleHit>,
    /// Carte des effets au format terminal.
    pub card: String,
    /// Vrai si la transaction a été signée puis exécutée.
    pub executed: bool,
}

/// Vue d'une entrée pour l'API JSON : les montants y sont des chaînes exactes.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JournalEntryView {
    /// Rang dans la frise.
    pub id: u64,
    /// Horodatage Unix, en secondes.
    pub timestamp: u64,
    /// `transfer` ou `function_call`.
    pub kind: String,
    /// Résumé de la demande, en une ligne.
    pub summary: String,
    /// Compte visé (contrat ou destinataire).
    pub target: String,
    /// Montant attaché en yoctoNEAR, en chaîne décimale.
    pub amount_yocto: String,
    /// Intention déclarée.
    pub intent_summary: String,
    /// Verdict du pare-feu.
    pub verdict: Verdict,
    /// Identifiants des règles qui ont mordu.
    pub rule_ids: Vec<String>,
    /// Carte des effets au format terminal.
    pub card: String,
    /// Vrai si la transaction a été exécutée.
    pub executed: bool,
}

impl JournalEntry {
    /// Vue JSON stable de l'entrée.
    pub fn view(&self) -> JournalEntryView {
        JournalEntryView {
            id: self.id,
            timestamp: self.timestamp,
            kind: kind_of(&self.demand).to_string(),
            summary: self.demand.summary(),
            target: target_of(&self.demand),
            amount_yocto: amount_of(&self.demand).to_string(),
            intent_summary: self.intent.summary.clone(),
            verdict: self.verdict,
            rule_ids: self.rules.iter().map(|r| r.id.clone()).collect(),
            card: self.card.clone(),
            executed: self.executed,
        }
    }
}

/// Type de demande, en `snake_case` comme le JSON du SPEC.
fn kind_of(demand: &Demande) -> &'static str {
    match demand {
        Demande::Transfer { .. } => "transfer",
        Demande::FunctionCall { .. } => "function_call",
    }
}

/// Compte visé par la demande.
fn target_of(demand: &Demande) -> String {
    match demand {
        Demande::Transfer { to, .. } => to.clone(),
        Demande::FunctionCall { contract, .. } => contract.clone(),
    }
}

/// Montant attaché à la demande.
fn amount_of(demand: &Demande) -> Yocto {
    demand.amount()
}

/// Frise en mémoire, protégée par un verrou de lecture/écriture.
#[derive(Debug, Default)]
pub struct Journal {
    entries: RwLock<Vec<JournalEntry>>,
}

impl Journal {
    /// Crée une frise vide.
    pub fn new() -> Self {
        Self::default()
    }

    /// Ajoute une entrée et la rend.
    pub fn record(
        &self,
        demand: Demande,
        intent: Intent,
        decision: &Decision,
        card: String,
        executed: bool,
    ) -> JournalEntry {
        let mut entries = self.lock();
        let entry = JournalEntry {
            id: entries.len() as u64 + 1,
            timestamp: now_seconds(),
            demand,
            intent,
            verdict: decision.verdict,
            rules: decision.rules.clone(),
            card,
            executed,
        };
        entries.push(entry.clone());
        entry
    }

    /// Copie des entrées, dans l'ordre d'arrivée.
    pub fn entries(&self) -> Vec<JournalEntry> {
        self.lock().clone()
    }

    /// Vues JSON des entrées, dans l'ordre d'arrivée.
    pub fn views(&self) -> Vec<JournalEntryView> {
        self.entries().iter().map(JournalEntry::view).collect()
    }

    /// Vide la frise (utile dans les tests).
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Prend le verrou d'écriture, en tolérant un empoisonnement.
    fn lock(&self) -> std::sync::RwLockWriteGuard<'_, Vec<JournalEntry>> {
        match self.entries.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

/// Heure courante en secondes depuis l'époque Unix.
fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}