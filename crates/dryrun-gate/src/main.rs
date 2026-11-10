//! Point d'entrée du garde-barrière Dry Run.
//!
//! Le CLI ne fait qu'orchestrer : toute la logique de décision vit dans
//! `Gatekeeper` (simulation → verdict → signature ou refus).

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent, Verdict};
use dryrun_gate::cli::{self, Command};
use dryrun_gate::gate::{Gatekeeper, Outcome};
use dryrun_gate::http::HttpServer;
use dryrun_gate::VaultStatus;

/// Dossier des wasm compilés, relatif à la racine du dépôt.
const DEFAULT_WASM_DIR: &str = "contracts/target/wasm32-unknown-unknown/release";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = match cli::parse(&args) {
        Ok(cmd) => cmd,
        Err(err) => {
            eprintln!("dryrun-gate : {err}");
            print_help();
            std::process::exit(2);
        }
    };

    match command {
        Command::Help => print_help(),
        other => run(other).await?,
    }
    Ok(())
}

/// Exécute une commande qui a besoin du garde-barrière.
async fn run(command: Command) -> anyhow::Result<()> {
    let gate = Gatekeeper::start(Path::new(DEFAULT_WASM_DIR)).await?;
    match command {
        Command::Propose { demand, intent } => propose(&gate, demand, intent).await,
        Command::Approve { contract } => approve(&gate, &contract).await,
        Command::Freeze => {
            gate.freeze().await?;
            println!("Coffre gelé. Toute demande ultérieure sera refusée (ERR_FROZEN).");
            status(&gate).await
        }
        Command::Status => status(&gate).await,
        Command::Log => show_log(&gate),
        Command::Serve { port, web_dir } => serve(gate, port, web_dir).await,
        Command::Help => Ok(()),
    }
}

/// Traite une proposition : carte, puis refus, confirmation ou signature.
async fn propose(gate: &Gatekeeper, demand: Demande, intent: Intent) -> anyhow::Result<()> {
    let outcome = gate.propose(&demand, &intent).await?;
    println!("{}", outcome.card);
    println!("{}", outcome.note);

    match outcome.verdict {
        Verdict::Block => {
            print_rules(&outcome);
            println!("Aucune transaction n'a touché la chaîne.");
            std::process::exit(1);
        }
        Verdict::Allow => println!("Exécuté : {}", outcome.executed),
        Verdict::Warn => {
            print_rules(&outcome);
            confirm_warn(gate, &demand, &intent).await?;
        }
    }
    Ok(())
}

/// Demande confirmation au propriétaire, puis exécute si accepté.
async fn confirm_warn(
    gate: &Gatekeeper,
    demand: &Demande,
    intent: &Intent,
) -> anyhow::Result<()> {
    if !ask_yes_no("Confirmer malgré l'avertissement ? [yes/no] ") {
        println!("Refusé par le propriétaire : rien n'a été signé.");
        return Ok(());
    }

    // Un contrat inconnu doit d'abord être approuvé. En démonstration, la clé
    // du propriétaire vit dans ce processus ; en production, cette étape se
    // fait depuis le téléphone du propriétaire (voir web/README.md).
    if let Some(contract) = demand.target() {
        let known = gate.status().await?.known_contracts;
        if !known.contains_key(contract) {
            println!("Contrat inconnu : approbation du propriétaire pour {contract}.");
            gate.approve_contract(contract, "owner-approved").await?;
        }
    }

    // Deuxième simulation, avec le contrat désormais connu.
    let second = gate.propose(demand, intent).await?;
    println!("{}", second.card);
    println!("{}", second.note);
    print_rules(&second);
    if second.executed {
        println!("Transaction signée et exécutée.");
    }
    Ok(())
}

/// Affiche les règles qui ont mordu.
fn print_rules(outcome: &Outcome) {
    if outcome.rules.is_empty() {
        println!("règles : aucune");
        return;
    }
    println!("règles en cause :");
    for rule in &outcome.rules {
        println!(
            "  {} {} — {} ({})",
            rule.id,
            rule.title,
            rule.detail,
            rule.severity.as_str()
        );
    }
}

/// Pose une question fermée sur l'entrée standard.
fn ask_yes_no(question: &str) -> bool {
    print!("{question}");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(answer.trim().to_lowercase().as_str(), "yes" | "y" | "oui" | "o")
}

/// `approve <contrat>` : réservé au propriétaire, jamais à l'agent.
async fn approve(gate: &Gatekeeper, contract: &str) -> anyhow::Result<()> {
    gate.approve_contract(contract, "owner-approved").await?;
    println!("Contrat {contract} approuvé (empreinte enregistrée par le propriétaire).");
    Ok(())
}

/// Affiche l'état du coffre, en grand.
async fn status(gate: &Gatekeeper) -> anyhow::Result<()> {
    let st: VaultStatus = gate.status().await?;
    println!("=== coffre {} ===", st.account_id);
    println!("solde        : {} NEAR", st.balance.near());
    println!("plafond tx   : {} NEAR", st.tx_cap.near());
    println!("plafond 24 h : {} NEAR", st.daily_cap.near());
    println!("dépensé 24 h : {} NEAR", st.spent_24h.near());
    println!("état         : {}", if st.is_frozen { "GELÉ" } else { "ouvert" });
    println!("propriétaire : {}", st.owner);
    println!("contrats     : {} connu(s)", st.known_contracts.len());
    Ok(())
}

/// Affiche la frise des demandes.
fn show_log(gate: &Gatekeeper) -> anyhow::Result<()> {
    let entries = gate.log();
    if entries.is_empty() {
        println!("journal vide.");
        return Ok(());
    }
    for entry in entries {
        println!(
            "#{:<3} {:<6} {:<40} exécuté={}",
            entry.id,
            entry.verdict.as_str(),
            entry.demand.summary(),
            entry.executed
        );
    }
    Ok(())
}

/// Démarre le serveur HTTP de la page web.
async fn serve(gate: Gatekeeper, port: u16, web_dir: String) -> anyhow::Result<()> {
    let web_dir = resolve_web_dir(&web_dir);
    let server = Arc::new(HttpServer::new(Arc::new(gate), web_dir));
    server.run(&format!("127.0.0.1:{port}")).await
}

/// Trouve le dossier `web/` que l'on soit à la racine ou dans un crate.
fn resolve_web_dir(requested: &str) -> std::path::PathBuf {
    let candidate = std::path::PathBuf::from(requested);
    if candidate.is_dir() {
        return candidate;
    }
    for fallback in ["../../web", "../web", "web"] {
        let path = std::path::PathBuf::from(fallback);
        if path.is_dir() {
            return path;
        }
    }
    candidate
}

/// Aide de la ligne de commande.
fn print_help() {
    println!("Dry Run — garde-barrière à simulation (pare-feu avant signature)");
    println!();
    println!("  dryrun-gate propose --to <compte> --amount <NEAR> --intent \"<texte>\"");
    println!("  dryrun-gate propose --call <contrat> --method <m> [--args <json>] \\");
    println!("               [--deposit <NEAR>] --intent \"<texte>\" \\");
    println!("               [--beneficiary <compte>]... [--min-in <jeton>:<montant>]...");
    println!("  dryrun-gate approve <contrat>");
    println!("  dryrun-gate freeze");
    println!("  dryrun-gate status");
    println!("  dryrun-gate log");
    println!("  dryrun-gate serve [--port 8787] [--web-dir web]");
}