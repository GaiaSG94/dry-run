//! Évaluation du banc robotisé : 200 cas (100 honnêtes, 100 attaques).
//!
//! Deux modes :
//! - **rapide** (par défaut) : le moteur de règles est jugé sur des effets
//!   **synthétisés** pour chaque cas, sans bac à sable. C'est ce que mesure la
//!   matrice de confusion : la logique de décision, pas la latence réseau.
//! - **réel** (`--real`) : chaque cas est rejoué dans le bac à sable via le
//!   simulateur. Plus lent (~5 min), réservé aux vérifications finales.
//!
//! Métriques publiées : vrais/faux positifs, vrais/faux négatifs, précision,
//! rappel, F1, latence médiane.

pub mod corpus;
pub mod synth;

use std::time::Instant;

use corpus::{Case, Expected};
use dryrun_core::judge;
use dryrun_core::rules::RuleCtx;
use dryrun_core::types::Verdict;

/// Rapport synthétique du banc de test.
#[derive(Clone, Debug, serde::Serialize)]
pub struct BenchReport {
    pub total: usize,
    pub attacks: usize,
    pub legitimate: usize,
    pub true_positives: usize,
    pub true_negatives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub median_latency_us: u128,
    pub p95_latency_us: u128,
    /// Les cas manqués, pour diagnostic (famille + identifiant).
    pub missed: Vec<String>,
}

/// Exécute le banc en mode rapide (effets synthétisés, moteur de règles réel).
pub fn run_bench(seed: u64) -> BenchReport {
    let cases = corpus::corpus(seed);
    let mut tp = 0usize;
    let mut tn = 0usize;
    let mut fp = 0usize;
    let mut fn_count = 0usize;
    let mut latencies: Vec<u128> = Vec::with_capacity(cases.len());
    let mut missed = Vec::new();

    for case in &cases {
        let (ctx, effects) = synth::scenario(case);
        let start = Instant::now();
        let decision = judge(&ctx, &effects);
        latencies.push(start.elapsed().as_micros());

        let held = decision.verdict != Verdict::Allow;
        match (case.expected, held) {
            (Expected::Attack, true) => tp += 1,
            (Expected::Attack, false) => {
                fn_count += 1;
                missed.push(describe(case));
            }
            (Expected::Legitimate, false) => tn += 1,
            (Expected::Legitimate, true) => {
                fp += 1;
                missed.push(describe(case));
            }
        }
    }

    report(cases.len(), tp, tn, fp, fn_count, latencies, missed)
}

/// Construit le rapport à partir des compteurs bruts.
fn report(
    total: usize,
    tp: usize,
    tn: usize,
    fp: usize,
    fn_count: usize,
    mut latencies: Vec<u128>,
    missed: Vec<String>,
) -> BenchReport {
    latencies.sort_unstable();
    let attacks = tp + fn_count;
    let legitimate = tn + fp;
    let precision = ratio(tp, tp + fp);
    let recall = ratio(tp, attacks);
    let f1_score = if precision + recall > 0.0 {
        2.0 * precision * recall / (precision + recall)
    } else {
        0.0
    };
    BenchReport {
        total,
        attacks,
        legitimate,
        true_positives: tp,
        true_negatives: tn,
        false_positives: fp,
        false_negatives: fn_count,
        precision,
        recall,
        f1_score,
        median_latency_us: percentile(&latencies, 50),
        p95_latency_us: percentile(&latencies, 95),
        missed,
    }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        1.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn percentile(sorted: &[u128], pct: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let index = (sorted.len() * pct / 100).min(sorted.len() - 1);
    sorted[index]
}

fn describe(case: &Case) -> String {
    format!(
        "#{} {} — {}",
        case.id,
        case.family.label(),
        case.intent.summary
    )
}

/// Affiche la matrice formatée dans le terminal.
pub fn print_report(rep: &BenchReport) {
    let line = "═".repeat(70);
    println!("\n╔{line}╗");
    println!(
        "║  DRY RUN — BANC DE 200 ROBOTS : MATRICE DE CONFUSION{}║",
        " ".repeat(15)
    );
    println!("╠{line}╣");
    row("cas testés", &rep.total.to_string());
    row("attaques dans le corpus", &rep.attacks.to_string());
    row("demandes légitimes", &rep.legitimate.to_string());
    println!("╠{line}╣");
    row(
        "vrais positifs (attaques bloquées)",
        &rep.true_positives.to_string(),
    );
    row(
        "vrais négatifs (légitimes acceptées)",
        &rep.true_negatives.to_string(),
    );
    row(
        "faux positifs (légitimes bloquées)",
        &rep.false_positives.to_string(),
    );
    row(
        "faux négatifs (attaques manquées)",
        &rep.false_negatives.to_string(),
    );
    println!("╠{line}╣");
    row("précision", &format!("{:.1} %", rep.precision * 100.0));
    row("rappel", &format!("{:.1} %", rep.recall * 100.0));
    row("score F1", &format!("{:.4}", rep.f1_score));
    row("latence médiane", &format!("{} µs", rep.median_latency_us));
    row("latence p95", &format!("{} µs", rep.p95_latency_us));
    println!("╚{line}╝");

    if !rep.missed.is_empty() {
        println!("\nCas à revoir :");
        for case in &rep.missed {
            println!("  - {case}");
        }
    }
}

fn row(label: &str, value: &str) {
    let width = 70 - label.chars().count() - value.chars().count() - 4;
    println!("║  {}{} {}║", label, " ".repeat(width.max(1)), value);
}

/// Sauvegarde le rapport en JSON (pour le README et le post de soumission).
pub fn save_json(rep: &BenchReport, path: &std::path::Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(rep)?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Contexte de jugement par défaut pour un cas du banc.
pub fn base_ctx(case: &Case) -> RuleCtx {
    RuleCtx {
        vault: "vault.test.near".to_string(),
        demand: case.demand.clone(),
        intent: case.intent.clone(),
        daily_spent: case.spent_24h,
        ..RuleCtx::default()
    }
}
