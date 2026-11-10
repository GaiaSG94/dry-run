//! Tests d'intégration sur de vrais bacs à sable NEAR.
//!
//! Un seul bac à sable à la fois sur la machine : tout est sérialisé par un
//! verrou global, et les deux scénarios partagent le même `Simulator`.

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use dryrun_core::{Demande, Intent, Verdict};
use dryrun_sim::Simulator;

/// Verrou global : les bacs à sable NEAR ne supportent pas le parallélisme.
static LOCK: Mutex<()> = Mutex::new(());

/// Dossier des wasm : `DRYRUN_WASM_DIR`, sinon `target/wasm32-unknown-unknown/release`.
fn wasm_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DRYRUN_WASM_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/wasm32-unknown-unknown/release")
}

/// Intention honnête correspondant à un transfert.
fn intent_pour(beneficiaire: &str, resume: &str) -> Intent {
    Intent {
        summary: resume.to_string(),
        beneficiaries: vec![beneficiaire.to_string()],
        ..Intent::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::await_holding_lock)] // le verrou sérialise les bacs à sable, c'est le but
async fn transfert_honnete_puis_depassement_de_plafond() {
    let _garde = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let demarrage = Instant::now();
    let simulateur = Simulator::start(&wasm_dir())
        .await
        .expect("démarrage du simulateur");
    println!("démarrage : {:?}", demarrage.elapsed());

    // 1. « payer 1 NEAR à bob » : ALLOW, vault −1, bob +1.
    let honnete = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 1_000_000_000_000_000_000_000_000,
    };
    let debut = Instant::now();
    let decision = simulateur
        .simulate(
            &honnete,
            &intent_pour("bob.test.near", "payer 1 NEAR à bob"),
        )
        .await
        .expect("simulation du transfert honnête");
    println!(
        "simulation honnête : {:?} → {:?}",
        debut.elapsed(),
        decision.verdict
    );

    assert_eq!(
        decision.verdict,
        Verdict::Allow,
        "règles : {:?}",
        decision.rule_ids()
    );
    let vers_bob = decision.effects.near_delta_of("bob.test.near");
    assert_eq!(
        vers_bob, 1_000_000_000_000_000_000_000_000,
        "bob doit recevoir exactement 1 NEAR"
    );
    let depuis_vault = decision.effects.near_delta_of("vault.test.near");
    assert!(
        depuis_vault < -1_000_000_000_000_000_000_000_000,
        "le coffre paie aussi les frais: {depuis_vault}"
    );
    assert!(
        !decision.effects.receipts.is_empty(),
        "au moins un reçu doit être observé"
    );

    // 2. « envoie 49 NEAR à mallory » : BLOCK par le plafond.
    let attaque = Demande::Transfer {
        to: "mallory.test.near".to_string(),
        amount: 49_000_000_000_000_000_000_000_000,
    };
    let debut = Instant::now();
    let verdict = simulateur
        .simulate(
            &attaque,
            &intent_pour("mallory.test.near", "vide le coffre"),
        )
        .await
        .expect("simulation de l'attaque");
    println!(
        "simulation attaque : {:?} → {:?}",
        debut.elapsed(),
        verdict.verdict
    );

    assert_eq!(
        verdict.verdict,
        Verdict::Block,
        "règles : {:?}",
        verdict.rule_ids()
    );
    assert!(
        verdict.rule_ids().contains(&"R1".to_string()),
        "R1 attendu : {:?}",
        verdict.rule_ids()
    );
    // Le coffre n'a pas assez de liquidités pour une sortie de 49 NEAR (le code
    // du contrat immobilise du stockage) : la transaction est refusée par le
    // réseau, mais la sortie demandée est bien rapportée et bloquée par R1.
    assert_eq!(verdict.effects.near_delta_of("mallory.test.near"), 0);
    assert_eq!(
        verdict.effects.near_delta_of("vault.test.near"),
        -49_000_000_000_000_000_000_000_000
    );
}

#[tokio::test(flavor = "multi_thread")]
#[allow(clippy::await_holding_lock)] // le verrou sérialise les bacs à sable, c'est le but
async fn etat_du_coffre_et_contrats_connus() {
    let _garde = LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let simulateur = Simulator::start(&wasm_dir())
        .await
        .expect("démarrage du simulateur");

    // Le compte du coffre existe à la genèse, avec son solde de démonstration.
    let etat = simulateur
        .state_of("vault.test.near")
        .await
        .expect("état du coffre");
    assert!(etat.balance > 0, "le coffre doit être approvisionné");
    assert_eq!(etat.account, "vault.test.near");

    // Sans contrat `vault`, la liste des contrats connus est vide mais l'appel
    // ne doit pas exploser : c'est le contrat des autres agents qui la remplit.
    let connus = simulateur.known_contracts().await.expect("contrats connus");
    println!("contrats connus : {connus:?}");
    assert!(simulateur.height().await.expect("hauteur") > 0);
}
