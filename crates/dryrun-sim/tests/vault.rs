//! Test d'intégration du coffre : déploiement verrouillé, exécution réelle, refus.

use std::path::PathBuf;

use dryrun_core::Demande;
use dryrun_sim::Simulator;

/// Dossier des wasm : `DRYRUN_WASM_DIR`, sinon la cible de compilation des contrats.
fn wasm_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("DRYRUN_WASM_DIR") {
        return PathBuf::from(dir);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/wasm32-unknown-unknown/release")
}

#[tokio::test(flavor = "multi_thread")]
async fn le_coffre_est_verrouille_et_execute_par_la_cle_d_agent() {
    if !wasm_dir().join("vault.wasm").exists() {
        println!("vault.wasm absent : test ignoré (le contrat est écrit par un autre agent)");
        return;
    }
    // `start` déploie déjà ; `deploy_all` doit rester idempotent.
    let simulateur = Simulator::start(&wasm_dir()).await.expect("démarrage");
    let deploye = simulateur
        .deploy_all()
        .await
        .expect("second déploiement (idempotent)");
    println!("contrats : {:?}", deploye.contracts);
    println!("clés du coffre : {:?}", deploye.vault_keys);
    assert!(
        deploye.contracts.contains_key("vault.test.near"),
        "le coffre doit être déployé"
    );
    assert!(
        deploye.vault_locked,
        "le coffre ne doit plus avoir de clé FullAccess"
    );
    let portees: Vec<&str> = deploye
        .vault_keys
        .iter()
        .map(|(_, scope)| scope.as_str())
        .collect();
    assert!(
        portees.contains(&"function_call:execute"),
        "clé d'agent absente : {portees:?}"
    );
    assert!(
        portees.contains(&"function_call:freeze"),
        "clé de gel absente : {portees:?}"
    );
    assert!(
        !portees.contains(&"full_access"),
        "clé plein accès encore présente"
    );

    // 2. L'état du coffre est lisible et conforme à la SPEC.
    let statut = simulateur.vault_status().await.expect("statut du coffre");
    println!("statut : {:?}", statut.raw);
    assert_eq!(statut.owner().as_deref(), Some("owner.test.near"));
    assert!(!statut.frozen(), "le coffre neuf n'est pas gelé");

    // 3. Exécution réelle par la clé d'agent : 1 NEAR à bob.
    let honnete = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 1_000_000_000_000_000_000_000_000,
    };
    let issue = simulateur
        .execute(&honnete)
        .await
        .expect("exécution honnête");
    println!("issue exécution : {issue:?}");
    assert!(issue.success, "exécution refusée : {:?}", issue.message);

    // 4. Au-delà du plafond : le coffre refuse on-chain, même sans simulateur.
    let attaque = Demande::Transfer {
        to: "mallory.test.near".to_string(),
        amount: 49_000_000_000_000_000_000_000_000,
    };
    let refus = simulateur
        .execute(&attaque)
        .await
        .expect("exécution de l'attaque");
    println!("refus : {refus:?}");
    assert!(!refus.success, "le coffre doit refuser 49 NEAR");
    assert_eq!(refus.refusal.as_deref(), Some("ERR_OVER_TX_CAP"));

    // 5. Gel d'un geste : la clé de gel ne peut que geler (I7), et le coffre
    //    refuse ensuite toute sortie (I2).
    let gel = simulateur.freeze().await.expect("gel");
    println!("gel : {gel:?}");
    assert!(
        gel.success,
        "le gel doit passer par la clé de gel : {:?}",
        gel.message
    );
    assert!(simulateur
        .vault_status()
        .await
        .expect("statut après gel")
        .frozen());
    let apres_gel = simulateur
        .execute(&honnete)
        .await
        .expect("exécution après gel");
    assert!(!apres_gel.success, "un coffre gelé ne sort plus rien");
    assert_eq!(apres_gel.refusal.as_deref(), Some("ERR_FROZEN"));
}
