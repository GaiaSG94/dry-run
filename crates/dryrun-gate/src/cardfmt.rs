//! Formatage de la carte des effets : terminal et structure JSON stable.

use dryrun_core::card::{format_ynear, render};
use dryrun_core::types::{Decision, Intent, RuleHit, Verdict};
use serde::{Deserialize, Serialize};

/// Variation de solde NEAR, prête à afficher.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NearDeltaView {
    /// Compte dont le solde bouge.
    pub account: String,
    /// Variation en yoctoNEAR, en chaîne décimale (précision exacte côté web).
    pub delta_yocto: String,
    /// Vrai si la variation est négative.
    pub is_negative: bool,
    /// Variation déjà formatée en NEAR.
    pub formatted: String,
}

/// Variation de jeton fongible, prête à afficher.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FtDeltaView {
    /// Compte du jeton (NEP-141).
    pub token: String,
    /// Compte dont le solde bouge.
    pub account: String,
    /// Variation en unités de base du jeton, en chaîne décimale.
    pub delta: String,
}

/// Résumé des reçus observés.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptsView {
    /// Nombre de reçus.
    pub total: usize,
    /// Nombre de reçus en échec.
    pub failed: usize,
    /// Vrai si des fonds sont sortis avant un échec.
    pub stranded: bool,
}

/// Règle déclenchée, prête à afficher.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleHitView {
    /// Identifiant, par exemple `R8`.
    pub id: String,
    /// Titre court.
    pub title: String,
    /// Détail chiffré.
    pub detail: String,
    /// Gravité de la règle.
    pub severity: Verdict,
}

/// Carte des effets en JSON stable, consommée par la page web.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CardView {
    /// Intention déclarée, ou mention explicite d'absence.
    pub summary: String,
    /// Verdict du pare-feu.
    pub verdict: Verdict,
    /// Verdict en majuscules (`ALLOW`, `WARN`, `BLOCK`).
    pub verdict_str: String,
    /// Variations de solde NEAR.
    pub near_deltas: Vec<NearDeltaView>,
    /// Variations de jetons fongibles.
    pub ft_deltas: Vec<FtDeltaView>,
    /// Clés et approbations observées, en une ligne chacune.
    pub keys: Vec<String>,
    /// Résumé des reçus.
    pub receipts: ReceiptsView,
    /// Règles déclenchées.
    pub rules: Vec<RuleHitView>,
    /// Hauteur du bloc simulé.
    pub height: u64,
    /// Gas brûlé pendant la simulation.
    pub gas_burnt: u64,
    /// Carte rendue au format terminal (celle que lit le jury).
    pub rendered_terminal: String,
}

/// Carte au format terminal (celle de `dryrun_core::card::render`).
pub fn format_terminal(decision: &Decision, intent: &Intent) -> String {
    render(decision, intent)
}

/// Carte au format JSON stable.
pub fn format_json(decision: &Decision, intent: &Intent) -> CardView {
    let near_deltas = decision
        .effects
        .near_delta
        .iter()
        .map(|delta| NearDeltaView {
            account: delta.account.clone(),
            delta_yocto: delta.delta.to_string(),
            is_negative: delta.delta < 0,
            formatted: format!("{:+}", format_ynear(delta.delta)),
        })
        .collect();

    let ft_deltas = decision
        .effects
        .ft_delta
        .iter()
        .map(|delta| FtDeltaView {
            token: delta.token.clone(),
            account: delta.account.clone(),
            delta: delta.delta.to_string(),
        })
        .collect();

    CardView {
        summary: summary_of(intent),
        verdict: decision.verdict,
        verdict_str: decision.verdict.as_str().to_string(),
        near_deltas,
        ft_deltas,
        keys: key_lines(decision),
        receipts: ReceiptsView {
            total: decision.effects.receipts.len(),
            failed: decision
                .effects
                .receipts
                .iter()
                .filter(|r| r.status == "failed")
                .count(),
            stranded: decision.effects.stranded,
        },
        rules: decision.rules.iter().map(rule_view).collect(),
        height: decision.effects.snapshot.height,
        gas_burnt: decision.effects.gas_burnt,
        rendered_terminal: render(decision, intent),
    }
}

/// Résumé lisible de l'intention déclarée.
fn summary_of(intent: &Intent) -> String {
    let summary = intent.summary.trim();
    if summary.is_empty() {
        "opération sans intention déclarée".to_string()
    } else {
        summary.to_string()
    }
}

/// Une ligne par clé ou approbation observée.
fn key_lines(decision: &Decision) -> Vec<String> {
    let keys = decision.effects.keys.iter().map(|k| format!("{k:?}"));
    let nft = decision.effects.nft.iter().map(|n| format!("{n:?}"));
    keys.chain(nft).collect()
}

/// Convertit une règle en vue affichable.
fn rule_view(rule: &RuleHit) -> RuleHitView {
    RuleHitView {
        id: rule.id.clone(),
        title: rule.title.clone(),
        detail: rule.detail.clone(),
        severity: rule.severity,
    }
}
