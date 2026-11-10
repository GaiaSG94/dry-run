//! Simulation différentielle : démasquer les contrats à double visage.
//!
//! La même demande est jouée deux fois, au montant réel et au centième. Un
//! contrat qui se comporte différemment selon le montant — ou qui détecte le
//! bac à sable et se tient bien sur les petits montants — trahit sa divergence
//! par un taux d'échange incohérent (règle R12).

use dryrun_core::types::{Effects, RuleHit, Verdict};
use dryrun_core::{Demande, Intent};

/// Écart de taux toléré entre les deux passages, en pour mille.
pub const TOLERANCE_PERMILLE: i128 = 100;

/// Rapport de la comparaison des deux passages.
#[derive(Clone, Debug, Default)]
pub struct DifferentialReport {
    /// Taux reçu/dépensé au montant réel, en pour mille.
    pub rate_full: i128,
    /// Taux reçu/dépensé au montant réduit, en pour mille.
    pub rate_small: i128,
    /// Écart relatif entre les deux taux, en pour mille.
    pub drift_permille: i128,
    /// Les deux passages ont-ils divergé au-delà de la tolérance ?
    pub divergent: bool,
    /// Un des deux passages n'a rien reçu alors que l'autre recevait quelque chose.
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
pub fn compare(
    vault: &str,
    intent: &Intent,
    full: &Effects,
    small: &Effects,
) -> DifferentialReport {
    let token = intent
        .min_in
        .first()
        .map(|(name, _)| name.as_str())
        .unwrap_or("");
    let rate_full = rate_permille(full, vault, token);
    let rate_small = rate_permille(small, vault, token);
    let asymmetric = (rate_full == 0) != (rate_small == 0);
    let drift = if rate_full == 0 || rate_small == 0 {
        if asymmetric {
            i128::MAX / 2
        } else {
            0
        }
    } else {
        ((rate_full - rate_small).abs() * 1000) / rate_small.max(1)
    };
    DifferentialReport {
        rate_full,
        rate_small,
        drift_permille: drift,
        divergent: drift > TOLERANCE_PERMILLE,
        asymmetric,
    }
}

/// Taux reçu pour mille dépensé, en pour mille.
fn rate_permille(eff: &Effects, vault: &str, token: &str) -> i128 {
    let out = eff.near_delta_of(vault).unsigned_abs();
    if out == 0 {
        return 0;
    }
    let received = if token.is_empty() {
        eff.near_delta
            .iter()
            .filter(|d| d.account == vault && d.delta > 0)
            .map(|d| d.delta)
            .sum::<i128>()
            .unsigned_abs()
    } else {
        eff.ft_delta_of(token, vault).unsigned_abs()
    };
    (received as i128).saturating_mul(1000) / (out as i128).max(1)
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
            "taux au montant réel {} ‰ contre {} ‰ au centième (écart {} ‰) : le contrat a deux visages",
            report.rate_full, report.rate_small, report.drift_permille
        ),
        severity: Verdict::Block,
    })
}
