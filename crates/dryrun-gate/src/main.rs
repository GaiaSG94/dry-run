//! Point d'entrée du garde-barrière Dry Run.
//!
//! Le CLI ne fait qu'orchestrer : toute la logique de décision vit dans
//! `Gatekeeper` (simulation → verdict → signature ou refus).

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent, Verdict};
use dryrun_gate::cli::{self, Command, Network};
use dryrun_gate::demo;
use dryrun_gate::gate::{Gatekeeper, Outcome};
use dryrun_gate::http::HttpServer;
use dryrun_gate::lang::Lang;
use dryrun_gate::{Simulator, VaultStatus};

/// Dossier des wasm compilés, relatif à la racine du dépôt.
const DEFAULT_WASM_DIR: &str = "target/wasm32-unknown-unknown/release";

/// Choix du simulateur, réglable par `--sim auto|real|stub`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SimChoice {
    /// Vrai simulateur, avec repli sur le bouchon s'il ne démarre pas.
    Auto,
    /// Vrai simulateur, sans repli.
    Real,
    /// Bouchon en mémoire (démonstration hors chaîne, tests).
    Stub,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (args, sim_choice) = extract_sim_choice(std::env::args().skip(1).collect());
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
        other => run(other, sim_choice).await?,
    }
    Ok(())
}

/// Retire l'option globale `--sim <mode>` des arguments.
fn extract_sim_choice(args: Vec<String>) -> (Vec<String>, SimChoice) {
    let mut kept = Vec::new();
    let mut choice = SimChoice::Auto;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--sim" {
            choice = match args.get(index + 1).map(String::as_str) {
                Some("stub") => SimChoice::Stub,
                Some("real") => SimChoice::Real,
                _ => SimChoice::Auto,
            };
            index += 2;
            continue;
        }
        kept.push(args[index].clone());
        index += 1;
    }
    (kept, choice)
}

/// Construit le garde-barrière selon le simulateur et le réseau demandés.
async fn open_gate(choice: SimChoice, network: &Network) -> anyhow::Result<Gatekeeper> {
    let gate = match network {
        Network::Local => match choice {
            SimChoice::Stub => Gatekeeper::from_sim(Arc::new(Simulator::stub().await?)),
            SimChoice::Real | SimChoice::Auto => {
                Gatekeeper::start(Path::new(DEFAULT_WASM_DIR)).await?
            }
        },
        Network::Chain {
            vault,
            name,
            rpc_url,
        } => {
            let sim = Simulator::testnet(vault, name, rpc_url).await?;
            Gatekeeper::from_sim(Arc::new(sim))
        }
    };
    // Niveau V1 : le propriétaire dépose la clé publique du simulateur sur le
    // coffre. À partir de là, le coffre n'exécute **que** des demandes
    // accompagnées d'un certificat signé et frais : voler la clé de l'agent ne
    // suffit plus. En démonstration locale, la clé du propriétaire vit dans ce
    // processus ; en production, c'est un geste du propriétaire (voir README).
    // Sur le testnet, aucune `sim_key` n'est enregistrée : le certificat n'y
    // est pas exigé, et la clé du compte signe directement l'action.
    if let Network::Local = network {
        match gate.install_sim_key().await {
            Ok(key) => println!("clé du simulateur enregistrée sur le coffre : {key}"),
            Err(err) => eprintln!("certificats inactifs ({err:#})"),
        }
    }
    Ok(gate)
}

/// Exécute une commande qui a besoin du garde-barrière.
async fn run(command: Command, sim_choice: SimChoice) -> anyhow::Result<()> {
    // `sim` n'a pas besoin de chaîne : il juge le moteur de règles sur un corpus
    // synthétique. On ne démarre donc pas le bac à sable pour rien.
    if let Command::Sim { seed } = command {
        return run_bench(seed);
    }
    let network = match &command {
        Command::Serve { network, .. } | Command::Mcp { network, .. } => network.clone(),
        _ => Network::local(),
    };
    let gate = open_gate(sim_choice, &network).await?;
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
        Command::Demo => demo::run_demo(&gate).await,
        Command::Attack => dryrun_gate::attack::run_attack(&gate).await,
        Command::Serve {
            port,
            web_dir,
            network,
        } => serve(gate, port, web_dir, &network).await,
        Command::Sim { seed } => run_bench(seed),
        Command::Mcp { network, lang } => {
            let where_ = match &network {
                Network::Local => "coffre local".to_string(),
                Network::Chain { vault, .. } => format!("coffre testnet {vault}"),
            };
            eprintln!(
                "serveur MCP Dry Run (JSON-RPC 2.0 sur stdin/stdout, langue : {}, {where_})",
                match lang {
                    Lang::En => "en",
                    Lang::Fr => "fr",
                }
            );
            dryrun_gate::mcp::run_mcp_server(Arc::new(gate), lang).await
        }
        Command::Help => Ok(()),
    }
}

/// Banc de test : 200 robots, matrice de confusion, rapport JSON.
fn run_bench(seed: u64) -> anyhow::Result<()> {
    let report = dryrun_gate::bench::run_bench(seed);
    dryrun_gate::bench::print_report(&report);
    dryrun_gate::bench::save_json(&report, std::path::Path::new("bench_matrix.json"))?;
    println!("Rapport sauvegardé dans bench_matrix.json");
    dryrun_gate::summary::line(
        "bench",
        &[
            ("cases", report.total.to_string()),
            ("attacks", report.attacks.to_string()),
            ("precision", format!("{:.1}", report.precision * 100.0)),
            ("recall", format!("{:.1}", report.recall * 100.0)),
            ("f1", format!("{:.4}", report.f1_score)),
            ("seed", seed.to_string()),
        ],
    );
    Ok(())
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
async fn confirm_warn(gate: &Gatekeeper, demand: &Demande, intent: &Intent) -> anyhow::Result<()> {
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
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        println!();
        println!("Mode non interactif : répondez « --yes » pour confirmer malgré l'avertissement.");
        return false;
    }
    print!("{question}");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    matches!(
        answer.trim().to_lowercase().as_str(),
        "yes" | "y" | "oui" | "o"
    )
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
    println!(
        "état         : {}",
        if st.is_frozen { "GELÉ" } else { "ouvert" }
    );
    println!("simulateur   : {}", st.backend);
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
async fn serve(
    gate: Gatekeeper,
    port: u16,
    web_dir: String,
    network: &Network,
) -> anyhow::Result<()> {
    let web_dir = resolve_web_dir(&web_dir);
    match network {
        Network::Local => println!("interface web accessible sur http://0.0.0.0:{port}/"),
        Network::Chain { vault, .. } => println!(
            "interface web accessible sur http://0.0.0.0:{port}/ — coffre testnet {vault} (RPC publique)"
        ),
    }
    let server = Arc::new(HttpServer::new(Arc::new(gate), web_dir));
    server.run(&format!("0.0.0.0:{port}")).await
}

/// Trouve le dossier `web/` que l'on soit à la racine ou dans un crate.
fn resolve_web_dir(requested: &str) -> std::path::PathBuf {
    let candidate = std::path::PathBuf::from(requested);
    if candidate.is_dir() {
        return candidate;
    }
    for fallback in ["web", "../../web", "../web"] {
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
    println!("  dryrun-gate serve [--port 8787] [--web-dir web] [--network local|testnet] [--vault <compte>]");
    println!("  dryrun-gate sim [graine]     # banc de 200 robots + matrice de confusion");
    println!("  dryrun-gate mcp [--network local|testnet] [--vault <compte>] [--lang en|fr]");
    println!();
    println!("Option globale : --sim auto|real|stub (défaut : auto).");
    println!("`stub` joue la démonstration sur un coffre en mémoire, sans bac à sable.");
    println!();
    println!("Mode testnet : --network testnet --vault <compte.testnet> branche le");
    println!("garde-barrière sur un coffre `vault` réellement déployé : `status()` est");
    println!("lu par RPC, les demandes ALLOW sont signées et envoyées sur la chaîne.");
    println!("La clé du compte reste dans le trousseau : la CLI NEAR la lit, jamais Dry Run.");
}
