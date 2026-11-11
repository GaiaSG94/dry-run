//! Tests d'intégration V1 complets contre la chaîne locale `near-sandbox` :
//! - le coffre n'exécute rien sans certificat dès que la clé du simulateur est posée ;
//! - une transaction accompagnée d'un certificat signé est acceptée et exécutée ;
//! - un certificat périmé est refusé on-chain (`ERR_CERT_STALE`) ;
//! - un certificat rejoué avec le même nonce est refusé on-chain (`ERR_CERT_USED`) ;
//! - une demande altérée ne correspond plus au digest (`ERR_CERT_MISMATCH`).

use dryrun_core::types::{Demande, Intent};
use dryrun_sim::accounts;
use dryrun_sim::Simulator;

fn wasm_dir() -> std::path::PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    if !dir.join("target/wasm32-unknown-unknown/release").is_dir() {
        if let Some(p) = dir.parent().and_then(|p| p.parent()) {
            dir = p.to_path_buf();
        }
    }
    dir.join("target/wasm32-unknown-unknown/release")
}

#[tokio::test]
async fn v1_e2e_certificat_exige_apres_installation_cle() -> anyhow::Result<()> {
    let wasm = wasm_dir();
    let sim = Simulator::start(&wasm).await?;
    sim.install_sim_key().await?;

    let demand = Demande::Transfer {
        to: accounts::BOB.to_string(),
        amount: 10u128.pow(24),
    };

    // 1. Appel sans certificat -> refus on-chain ERR_NO_CERT
    let sans_cert = sim.execute(&demand).await?;
    assert!(!sans_cert.success, "devait refuser sans certificat");
    assert_eq!(
        sans_cert.refusal.as_deref(),
        Some("ERR_NO_CERT"),
        "devait journaliser ERR_NO_CERT"
    );

    // 2. Simulation -> certificat signé -> exécution réussie
    let intent = Intent {
        summary: "payer 1 NEAR à bob".to_string(),
        beneficiaries: vec![accounts::BOB.to_string()],
        min_in: vec![],
        max_out: None,
    };
    let decision = sim.simulate(&demand, &intent).await?;
    assert_eq!(decision.verdict.as_str(), "ALLOW");

    let cert = sim.sign_cert(&demand, &decision.effects, vec![]).await?;
    let avec_cert = sim.execute_with_cert(&demand, Some(&cert)).await?;
    assert!(
        avec_cert.success,
        "la demande certifiée devait être acceptée : {:?}",
        avec_cert.message
    );

    // 3. Rejeu du même certificat -> refus ERR_CERT_USED
    let rejeu = sim.execute_with_cert(&demand, Some(&cert)).await?;
    assert!(!rejeu.success, "le rejeu devait être refusé");
    assert_eq!(rejeu.refusal.as_deref(), Some("ERR_CERT_USED"));

    Ok(())
}

#[tokio::test]
async fn v1_e2e_certificat_altere_donne_mismatch() -> anyhow::Result<()> {
    let wasm = wasm_dir();
    let sim = Simulator::start(&wasm).await?;
    sim.install_sim_key().await?;

    let demand = Demande::Transfer {
        to: accounts::BOB.to_string(),
        amount: 10u128.pow(24),
    };
    let intent = Intent {
        summary: "payer 1 NEAR à bob".to_string(),
        beneficiaries: vec![accounts::BOB.to_string()],
        min_in: vec![],
        max_out: None,
    };
    let decision = sim.simulate(&demand, &intent).await?;
    let mut cert = sim.sign_cert(&demand, &decision.effects, vec![]).await?;

    // On altère la demande exécutée (le voleur essaie d'utiliser le certificat pour mallory)
    let detournement = Demande::Transfer {
        to: accounts::MALLORY.to_string(),
        amount: 10u128.pow(24),
    };
    let issue = sim.execute_with_cert(&detournement, Some(&cert)).await?;
    assert!(!issue.success, "le détournement devait être refusé");
    assert_eq!(issue.refusal.as_deref(), Some("ERR_CERT_MISMATCH"));

    // On altère la hauteur d'expiration du certificat -> refus de fraîcheur
    cert.cert.expires_height = 0;
    let stale = sim.execute_with_cert(&demand, Some(&cert)).await?;
    assert!(!stale.success, "le certificat périmé devait être refusé");
    assert_eq!(stale.refusal.as_deref(), Some("ERR_CERT_STALE"));

    Ok(())
}
