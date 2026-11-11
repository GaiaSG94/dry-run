//! Test d'intégration V1 : le contrôle au retour (post-condition) gèle le coffre
//! et marque le contrat hostile quand la promesse du certificat n'est pas tenue.
//!
//! Scénario :
//! 1. le propriétaire approuve `liar.test.near` (contrat qui répond « succès »
//!    sans rien transférer) ;
//! 2. le simulateur signe un certificat exigeant au moins 1.9 TEST en retour ;
//! 3. la transaction est exécutée (le pare-feu est volontairement contourné :
//!    c'est exactement le cas d'un contrat « dormeur » qui trompe la simulation) ;
//! 4. au retour, le coffre constate que le solde promis n'est pas atteint :
//!    il gèle et marque `liar.test.near` hostile.

use std::path::PathBuf;

use dryrun_core::types::{Demande, Intent};
use dryrun_sim::Simulator;
use serde_json::json;

fn wasm_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    if !dir.join("target/wasm32-unknown-unknown/release").is_dir() {
        if let Some(root) = dir.parent().and_then(|p| p.parent()) {
            dir = root.to_path_buf();
        }
    }
    dir.join("target/wasm32-unknown-unknown/release")
}

#[tokio::test]
async fn le_contrôle_au_retour_gele_et_marque_hostile() -> anyhow::Result<()> {
    let wasm = wasm_dir();
    let sim = Simulator::start(&wasm).await?;
    sim.install_sim_key().await?;
    sim.approve_contract("liar.test.near").await?;

    let demande = Demande::FunctionCall {
        contract: "liar.test.near".to_string(),
        method: "trap".to_string(),
        args: json!({ "args": "" }),
        deposit: 2 * 10u128.pow(24),
        gas: 100_000_000_000_000,
    };
    let intention = Intent {
        summary: "échanger 2 NEAR contre au moins 1.9 TEST".to_string(),
        beneficiaries: vec!["liar.test.near".to_string()],
        min_in: vec![("TEST".to_string(), 19 * 10u128.pow(17))],
        max_out: None,
    };

    let decision = sim.simulate(&demande, &intention).await?;
    assert_eq!(
        decision.verdict.as_str(),
        "BLOCK",
        "le pare-feu voit le piège"
    );

    // On force l'exécution avec un certificat : c'est le scénario « dormeur ».
    let cert = sim
        .sign_cert(
            &demande,
            &decision.effects,
            vec![("test-token.test.near".to_string(), 19 * 10u128.pow(17))],
        )
        .await?;
    let issue = sim.execute_with_cert(&demande, Some(&cert)).await?;
    assert!(issue.success, "l'appel lui-même réussit (le contrat ment)");

    // Le contrôle au retour tourne dans les reçus suivants : on laisse passer
    // quelques blocs avant de relire l'état du coffre.
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let status = sim.vault_status().await?;
    assert!(status.frozen(), "le coffre doit être gelé après l'écart");

    let hostile: serde_json::Value = sim
        .view_call(
            "vault.test.near",
            "is_hostile",
            &json!({ "account_id": "liar.test.near" }),
        )
        .await?;
    assert_eq!(
        hostile,
        json!(true),
        "le contrat menteur doit être marqué hostile"
    );

    // Un contrat hostile ne peut plus être appelé, même avec un certificat neuf.
    let refus = sim.execute_with_cert(&demande, Some(&cert)).await?;
    assert!(!refus.success);
    assert_eq!(refus.refusal.as_deref(), Some("ERR_HOSTILE"));

    Ok(())
}
