//! Corpus d'attaques : chaque piège est joué dans **son** scénario.
//!
//! Le pare-feu ne gèle pas le coffre : il refuse **avant** de signer. Les six
//! pièges sont donc joués à travers le pare-feu (aucun ne passe), puis le
//! témoin « wallet nu » est joué une fois pour de vrai sur le contrat le plus
//! simple (`drainer`) : c'est la preuve que le corpus nuit réellement.
//!
//! Deux scénarios supplémentaires montrent la défense en profondeur :
//! - le vol de la clé d'agent, arrêté on-chain par les plafonds puis par le
//!   certificat de simulation (V1) ;
//! - le contrôle au retour (V1) : une exécution qui ne tient pas sa promesse
//!   gèle le coffre et marque le contrat hostile.

use crate::amount::Amount;
use crate::gate::Gatekeeper;
use dryrun_core::types::{Demande, Intent, Verdict};

/// Un NEAR, en yoctoNEAR.
const ONE_NEAR: u128 = 10u128.pow(24);

/// Un TEST, en unités de base (18 décimales).
const ONE_TEST: u128 = 10u128.pow(18);

/// Gas attaché à un appel de contrat (voir `demo.rs`).
const GAS_APPEL: u64 = 100_000_000_000_000;

/// Un piège du corpus.
struct Piege {
    nom: &'static str,
    contrat: &'static str,
    depot: u128,
    /// Minimum attendu en TEST (0 = aucun minimum déclaré).
    min_test: u128,
    /// Règle attendue.
    attendue: &'static str,
    /// Ce que le wallet nu perd, en une phrase.
    temoin: &'static str,
}

/// Le corpus des six contrats piégés du niveau M.
fn corpus() -> Vec<Piege> {
    vec![
        Piege {
            nom: "drainer",
            contrat: "drainer.test.near",
            depot: 2 * ONE_NEAR,
            min_test: 19 * ONE_TEST / 10,
            attendue: "R8",
            temoin: "2 NEAR encaissés, rien reçu",
        },
        Piege {
            nom: "skimmer",
            contrat: "skimmer.test.near",
            depot: 2 * ONE_NEAR,
            min_test: 16 * ONE_TEST / 10,
            attendue: "R7",
            temoin: "20 % des jetons détournés vers un tiers",
        },
        Piege {
            nom: "half-step",
            contrat: "half-step.test.near",
            depot: 2 * ONE_NEAR,
            min_test: 0,
            attendue: "R9",
            temoin: "fonds pris, puis reçu en échec",
        },
        Piege {
            nom: "liar",
            contrat: "liar.test.near",
            depot: 2 * ONE_NEAR,
            min_test: 19 * ONE_TEST / 10,
            attendue: "R8",
            temoin: "« succès » annoncé, aucun jeton reçu",
        },
        Piege {
            nom: "approver",
            contrat: "approver.test.near",
            depot: ONE_NEAR,
            min_test: 0,
            attendue: "R3",
            temoin: "NFT du coffre approuvé à un tiers",
        },
        Piege {
            nom: "shapeshifter",
            contrat: "shapeshifter.test.near",
            depot: 2 * ONE_NEAR,
            min_test: 0,
            attendue: "R6",
            temoin: "code remplacé après l'accord",
        },
    ]
}

/// Joue le corpus : pare-feu d'abord, puis témoin sur `drainer`.
pub async fn run_attack(gate: &Gatekeeper) -> anyhow::Result<()> {
    titre("Dry Run — corpus d'attaques");

    println!(
        "  {:<14} {:<10} {:<28} ce que perd un wallet nu",
        "contrat", "verdict", "règles"
    );
    println!("  {}", "-".repeat(94));

    let total = corpus().len();
    let mut arrets = 0;
    let mut attendues = 0;

    for piege in corpus() {
        // Le propriétaire accepte le contrat une fois : scénario le plus
        // défavorable au pare-feu (le contrat est connu et approuvé).
        let _ = gate.approve_contract(piege.contrat, "empreinte").await;
        if piege.nom == "shapeshifter" {
            // Cas particulier : le code change **après** l'accord. On le joue
            // en deux temps pour que R6 (empreinte changée) morde vraiment.
            let (verdict, regles) = scenario_shapeshifter(gate, &piege).await?;
            println!(
                "  {:<14} {:<10} {:<28} {}",
                piege.nom,
                verdict.as_str(),
                regles,
                piege.temoin
            );
            if verdict != Verdict::Allow {
                arrets += 1;
            }
            if regles.contains("R6") {
                attendues += 1;
            }
            continue;
        }

        let demande = demande_piege(&piege);
        let intention = intention_piege(&piege);
        let outcome = gate.propose(&demande, &intention).await?;

        let regles = regles(&outcome.rules);
        if outcome.verdict != Verdict::Allow {
            arrets += 1;
        }
        if regles.contains(&piege.attendue.to_string()) {
            attendues += 1;
        }
        println!(
            "  {:<14} {:<10} {:<28} {}",
            piege.nom,
            outcome.verdict.as_str(),
            regles,
            piege.temoin
        );
        if outcome.verdict == Verdict::Allow {
            println!(
                "    ⚠️  {} n'a déclenché aucune règle : vérifier le bootstrap et les effets",
                piege.nom
            );
        }
    }

    println!();
    println!("  {arrets}/{total} pièges arrêtés avant signature.");
    println!("  {attendues}/{total} fois la règle attendue a mordu.");

    temoin_wallet_nu(gate).await?;
    vol_de_cle(gate).await?;

    if arrets != total {
        anyhow::bail!("tous les pièges auraient dû être arrêtés");
    }
    Ok(())
}

/// Scénario `shapeshifter` : contrat honnête accepté, puis code remplacé.
///
/// 1. le propriétaire approuve l'empreinte actuelle ;
/// 2. une demande passe (contrat honnête) ;
/// 3. le contrat remplace son code ;
/// 4. la même demande est refusée par R6.
async fn scenario_shapeshifter(
    gate: &Gatekeeper,
    piege: &Piege,
) -> anyhow::Result<(Verdict, String)> {
    let demande = demande_piege(piege);
    let intention = intention_piege(piege);

    // 2. Premier passage : honnête, le pare-feu laisse passer.
    let avant = gate.propose(&demande, &intention).await?;

    // 3. Remplacement du code : on déploie le wasm d'un autre piège.
    let code = std::fs::read("target/wasm32-unknown-unknown/release/trap_liar.wasm")
        .or_else(|_| std::fs::read("target/wasm32-unknown-unknown/release/trap-liar.wasm"))?;
    gate.owner_call(
        piege.contrat,
        "deploy",
        &serde_json::json!({ "code": code }),
    )
    .await?;

    // 4. Deuxième passage : l'empreinte a changé, R6 doit mordre.
    let apres = gate.propose(&demande, &intention).await?;
    let regles = if apres.verdict == Verdict::Allow {
        format!("aucune (premier passage : {})", avant.verdict.as_str())
    } else {
        regles(&apres.rules)
    };
    Ok((apres.verdict, regles))
}

/// Témoin : le wallet nu perd réellement de l'argent sur `drainer`./// Témoin : le wallet nu perd réellement de l'argent sur `drainer`.
///
/// Joué **après** le corpus : le pare-feu n'a rien signé, donc le coffre est
/// encore plein. On appelle le contrat directement avec la clé de l'agent, sans
/// certificat, pour montrer ce que le pare-feu empêche.
async fn temoin_wallet_nu(gate: &Gatekeeper) -> anyhow::Result<()> {
    titre("Témoin : wallet nu contre `drainer` (sans pare-feu, sans certificat)");
    let demande = Demande::FunctionCall {
        contract: "drainer.test.near".to_string(),
        method: "trap".to_string(),
        args: serde_json::json!(""),
        deposit: ONE_NEAR / 10,
        gas: GAS_APPEL,
    };
    match gate.execute_without_firewall(&demande).await {
        Ok(outcome) => println!(
            "  exécution directe : {} ({})",
            if outcome.success {
                format!("{} NEAR perdus", Amount(demande.amount()).near())
            } else {
                "refusée par le coffre".to_string()
            },
            outcome.tx_hash
        ),
        Err(err) => println!("  exécution directe refusée : {err}"),
    }
    Ok(())
}

/// Vol de clé : la clé d'agent seule ne vaut rien (plafonds + certificat).
async fn vol_de_cle(gate: &Gatekeeper) -> anyhow::Result<()> {
    titre("Vol de la clé d'agent (défense en profondeur)");
    let gros = Demande::Transfer {
        to: "mallory.test.near".to_string(),
        amount: 49 * ONE_NEAR,
    };
    let petit = Demande::Transfer {
        to: "mallory.test.near".to_string(),
        amount: ONE_NEAR,
    };

    for (label, demande) in [
        ("49 NEAR d'un coup (plafond par transaction)", &gros),
        ("1 NEAR avec la clé volée (sans certificat)", &petit),
    ] {
        match gate.execute_without_firewall(demande).await {
            Ok(outcome) => println!(
                "  {label:<44} → {}",
                if outcome.success {
                    "EXÉCUTÉ — INCIDENT".to_string()
                } else {
                    outcome.tx_hash.clone()
                }
            ),
            Err(err) => println!("  {label:<44} → refusé ({err})"),
        }
    }
    Ok(())
}

/// Construit la demande correspondant à un piège.
fn demande_piege(piege: &Piege) -> Demande {
    Demande::FunctionCall {
        contract: piege.contrat.to_string(),
        method: "trap".to_string(),
        args: serde_json::json!(""),
        deposit: piege.depot,
        gas: GAS_APPEL,
    }
}

/// Construit l'intention déclarée par l'agent pour un piège.
fn intention_piege(piege: &Piege) -> Intent {
    let min_in = if piege.min_test == 0 {
        vec![]
    } else {
        vec![("TEST".to_string(), piege.min_test)]
    };
    Intent {
        summary: format!(
            "échanger {} NEAR sur {}",
            Amount(piege.depot).near(),
            piege.contrat
        ),
        beneficiaries: vec![piege.contrat.to_string()],
        min_in,
        max_out: None,
    }
}

/// Identifiants des règles qui ont mordu.
fn regles(hits: &[dryrun_core::types::RuleHit]) -> String {
    if hits.is_empty() {
        return "aucune".to_string();
    }
    hits.iter()
        .map(|hit| hit.id.clone())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Affiche un titre de section.
fn titre(texte: &str) {
    println!("\n=== {texte} ===\n");
}
