//! Simulation différentielle : démasquer les contrats à double visage.
//!
//! La même demande est jouée deux fois, au montant réel et au centième. Un
//! contrat qui se comporte différemment selon le montant — ou qui détecte le
//! bac à sable et se tient bien sur les petits montants — trahit sa divergence
//! par un taux d'échange incohérent (règle R12).

use dryrun_core::types::{Effects, RuleHit, Verdict};
use dryrun_core::Demande;

/// Écart de taux toléré entre les deux passages, en pour mille.
pub const TOLERANCE_PERMILLE: i128 = 100;

/// Rapport de la comparaison des deux passages.
#[derive(Clone, Debug, Default)]
pub struct DifferentialReport {
    /// Jetons reçus par le coffre au montant réel.
    pub rate_full: i128,
    /// Jetons reçus au montant réduit, ramenés au montant réel (`× diviseur`).
    pub rate_small: i128,
    /// Manque relatif du passage réel par rapport au passage réduit, en pour
    /// mille. Nul quand le passage réduit n'a rien produit (aucun signal).
    pub drift_permille: i128,
    /// Les deux passages ont-ils divergé au-delà de la tolérance ?
    pub divergent: bool,
    /// Le passage réel n'a rien rendu alors que le passage réduit rendait.
    pub asymmetric: bool,
}

/// Construit une demande clone au montant réduit d'un facteur donné.
pub fn scaled(demand: &Demande, divisor: u128) -> Demande {
    match demand {
        Demande::Transfer { to, amount } => Demande::Transfer {
            to: to.clone(),
            amount: amount.saturating_div(divisor),
        },
        Demande::FunctionCall {
            contract,
            method,
            args,
            deposit,
            gas,
        } => Demande::FunctionCall {
            contract: contract.clone(),
            method: method.clone(),
            args: args.clone(),
            deposit: deposit.saturating_div(divisor),
            gas: *gas,
        },
    }
}

/// Compare deux rapports d'effets et produit la règle R12 si nécessaire.
///
/// `divisor` est le facteur appliqué au second passage (voir [`scaled`]) : le
/// reçu du petit montant est **ramené au montant réel** avant comparaison. On
/// compare donc des quantités de jetons et non des taux : le résultat ne dépend
/// ni des décimales du jeton (18 pour TEST, 24 pour NEAR), ni des frais de gaz
/// qui grèvent le delta du coffre.
///
/// Le test est **orienté** : seul un contrat plus généreux sur le petit montant
/// que sur le montant réel trahit un double visage (il se tient bien tant qu'il
/// est observé, puis vole). Un rejeu réduit qui ne produit rien — dépôt minimum
/// non atteint, `min_out` absolu — n'apporte aucun signal et ne bloque pas.
pub fn compare(vault: &str, divisor: u128, full: &Effects, small: &Effects) -> DifferentialReport {
    let rate_full = received(full, vault) as i128;
    let rate_small = (received(small, vault) as i128).saturating_mul(divisor as i128);
    if rate_small == 0 {
        // Rien à comparer : le passage réduit n'a rien rendu au coffre.
        return DifferentialReport {
            rate_full,
            rate_small,
            ..DifferentialReport::default()
        };
    }
    let asymmetric = rate_full == 0;
    let drift = (rate_small - rate_full) * 1000 / rate_small;
    DifferentialReport {
        rate_full,
        rate_small,
        drift_permille: drift,
        divergent: drift > TOLERANCE_PERMILLE,
        asymmetric,
    }
}

/// Jetons fongibles entrés dans le coffre, tous jetons confondus.
///
/// L'unité n'a pas d'importance ici : seul compte l'écart entre les deux
/// passages, joués sur le même contrat et le même jeton.
fn received(eff: &Effects, vault: &str) -> u128 {
    eff.ft_delta
        .iter()
        .filter(|d| d.account == vault && d.delta > 0)
        .map(|d| d.delta.unsigned_abs())
        .sum()
}

/// Règle R12 si le rapport signale une divergence.
pub fn differential_hit(report: &DifferentialReport) -> Option<RuleHit> {
    if !report.divergent && !report.asymmetric {
        return None;
    }
    Some(RuleHit {
        id: "R12".into(),
        title: "simulation différentielle divergente".into(),
        detail: format!(
            "{} unités reçues au montant réel contre {} promises par le passage au centième (manque {} ‰) : le contrat a deux visages",
            report.rate_full, report.rate_small, report.drift_permille
        ),
        severity: Verdict::Block,
    })
}
