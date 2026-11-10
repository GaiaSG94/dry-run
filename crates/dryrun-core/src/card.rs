//! La carte des effets : ce que le propriétaire et le jury voient.
//!
//! Format repris du cahier des charges (section 7.14).

use crate::types::{Decision, Effects, Intent, Verdict};

/// Formate une décision en carte lisible dans un terminal.
pub fn render(decision: &Decision, intent: &Intent) -> String {
    let mut out = String::new();
    let summary = intent.summary.trim();
    let summary = if summary.is_empty() {
        "opération sans intention déclarée"
    } else {
        summary
    };
    out.push_str(&format!(" {}\n", summary));
    out.push_str(&format!(" {:<70} {}\n", "", decision.verdict.as_str()));
    out.push('\n');
    out.push_str(&balances(&decision.effects));
    out.push_str(&keys_approvals(&decision.effects));
    out.push_str(&receipts(&decision.effects));
    out.push_str(&rules(decision));
    out.push_str(&footer(&decision.effects));
    out
}

/// Les variations de solde, en une ligne par compte.
fn balances(eff: &Effects) -> String {
    let mut out = String::from(" soldes\n");
    for d in &eff.near_delta {
        if d.delta == 0 {
            continue;
        }
        out.push_str(&format!(
            "   {:<40} {:>+} NEAR\n",
            d.account,
            format_ynear(d.delta)
        ));
    }
    for d in &eff.ft_delta {
        if d.delta == 0 {
            continue;
        }
        out.push_str(&format!(
            "   {:<40} {:>+} {}\n",
            d.account, d.delta, d.token
        ));
    }
    if out.lines().count() == 1 {
        out.push_str("   (aucun mouvement)\n");
    }
    out
}

/// Les clés et approbations observées.
fn keys_approvals(eff: &Effects) -> String {
    let mut out = String::from(" clés, approbations\n");
    if eff.keys.is_empty() && eff.nft.is_empty() {
        out.push_str("   none\n");
        return out;
    }
    for k in &eff.keys {
        out.push_str(&format!("   {:?}\n", k));
    }
    for n in &eff.nft {
        out.push_str(&format!("   {:?}\n", n));
    }
    out
}

/// Les reçus et leur statut.
fn receipts(eff: &Effects) -> String {
    let mut out = String::from(" reçus\n");
    if eff.receipts.is_empty() {
        out.push_str("   aucun\n");
        return out;
    }
    let failed = eff.receipts.iter().filter(|r| r.status == "failed").count();
    out.push_str(&format!("   {}, {} échec(s)\n", eff.receipts.len(), failed));
    out
}

/// Les règles qui ont mordu.
fn rules(decision: &Decision) -> String {
    let mut out = String::from(" règles\n");
    if decision.rules.is_empty() {
        out.push_str("   aucune\n");
        return out;
    }
    for r in &decision.rules {
        out.push_str(&format!("   {} {} — {}\n", r.id, r.title, r.detail));
    }
    out
}

/// Le pied de carte : hauteur simulée et gas.
fn footer(eff: &Effects) -> String {
    format!(
        "\n simulé au bloc {} — gas {} — {}\n",
        eff.snapshot.height,
        eff.gas_burnt,
        if eff.stranded {
            "fonds coincés"
        } else {
            "aucun reçu en échec"
        }
    )
}

/// Formate des yoctoNEAR en NEAR avec six décimales.
pub fn format_ynear(yocto: i128) -> String {
    let neg = yocto < 0;
    let abs = yocto.unsigned_abs();
    let whole = abs / 10u128.pow(24);
    let frac = abs % 10u128.pow(24);
    format!(
        "{}{}.{:06}",
        if neg { "-" } else { "" },
        whole,
        frac / 10u128.pow(18)
    )
}

/// Résumé d'une ligne pour la frise.
pub fn one_line(decision: &Decision, intent: &Intent) -> String {
    format!(
        "{} — {} — {}",
        {
            let s = intent.summary.trim();
            if s.is_empty() {
                "opération".to_string()
            } else {
                s.to_string()
            }
        },
        decision.verdict.as_str(),
        decision
            .rules
            .iter()
            .map(|r| r.id.clone())
            .collect::<Vec<_>>()
            .join(",")
    )
}

/// Vrai si le verdict vaut `BLOCK`.
pub fn is_block(decision: &Decision) -> bool {
    decision.verdict == Verdict::Block
}
