//! Test de bout en bout du mode **testnet** : un vrai client MCP obtient
//! `BLOCK` sur 49 NEAR et `ALLOW` + une empreinte de transaction sur 0,1 NEAR.
//!
//! Ce test parle au **testnet public** : il signe de vraies transactions et
//! dépense de vrais testnet NEAR. Il est donc ignoré par défaut (`cargo test`
//! ne le lance pas) et s'active explicitement :
//!
//! ```bash
//! cargo test -p dryrun-gate --test testnet -- --ignored --nocapture
//! ```
//!
//! Prérequis :
//! - la CLI NEAR est installée et la clé du compte est dans le trousseau
//!   (`near-cli-rs`, `save-to-keychain`) ;
//! - le coffre `vault` est déployé sur le compte indiqué (2 NEAR / tx,
//!   10 NEAR / 24 h).

use std::sync::Arc;

use serde_json::json;

use dryrun_gate::gate::Gatekeeper;
use dryrun_gate::lang::Lang;
use dryrun_gate::mcp::handle_tool_call;
use dryrun_gate::Simulator;

/// Compte du coffre de démonstration sur le testnet.
const VAULT: &str = "dryrun-vault-1112.testnet";

/// Destinataire du paiement légitime : un compte testnet qui existe.
const PAYEE: &str = "testnet";

/// Construit un garde-barrière branché sur le coffre testnet.
async fn gate() -> anyhow::Result<Gatekeeper> {
    let sim = Simulator::testnet(VAULT, "testnet", "https://rpc.testnet.near.org").await?;
    Ok(Gatekeeper::from_sim(Arc::new(sim)))
}

/// Une demande MCP de transfert.
fn transfer(to: &str, amount_near: &str) -> serde_json::Value {
    json!({
        "intent": format!("payer {amount_near} NEAR à {to}"),
        "kind": "transfer",
        "to": to,
        "amount_near": amount_near
    })
}

#[tokio::test]
#[ignore = "parle au testnet public : signé et Depensé de vrais testnet NEAR"]
async fn mcp_testnet_bloque_49_near_et_paie_0_1_near() -> anyhow::Result<()> {
    let gate = gate().await?;

    // 1. L'état du coffre est lu par RPC, sur la vraie chaîne.
    let status = gate.status().await?;
    assert_eq!(status.account_id, VAULT);
    assert_eq!(status.backend, "testnet");
    assert!(status.balance.yocto() > 0, "le coffre doit avoir un solde");
    assert!(status.tx_cap.yocto() > 0, "le plafond par tx doit être lu");
    assert!(status.daily_cap.yocto() > 0, "le plafond 24 h doit être lu");

    // 2. 49 NEAR : le pare-feu bloque **avant** toute signature.
    let bloque = handle_tool_call(
        &gate,
        "wallet_propose",
        &transfer("mallory.test.near", "49"),
        Lang::En,
    )
    .await;
    assert!(bloque.contains("VERDICT: BLOCK"), "sortie : {bloque}");
    assert!(bloque.contains("R1"), "la règle R1 doit mordre : {bloque}");
    assert!(
        bloque.contains("nothing (rejected before signing)"),
        "rien ne doit être signé : {bloque}"
    );
    assert!(!bloque.contains("tx "), "aucune transaction : {bloque}");

    // 3. 0,1 NEAR : le pare-feu autorise, la transaction part sur la chaîne.
    let autorise =
        handle_tool_call(&gate, "wallet_propose", &transfer(PAYEE, "0.1"), Lang::En).await;
    assert!(autorise.contains("VERDICT: ALLOW"), "sortie : {autorise}");
    assert!(
        autorise.contains("NEAR testnet · tx "),
        "la réponse doit porter l'empreinte : {autorise}"
    );

    // 4. L'empreinte est bien celle d'une transaction de la chaîne.
    let hash = autorise
        .split("NEAR testnet · tx ")
        .nth(1)
        .expect("empreinte")
        .split_whitespace()
        .next()
        .expect("empreinte non vide")
        .to_string();
    assert!(hash.len() >= 32, "empreinte trop courte : {hash}");

    // 5. La dépense est comptée sur la chaîne.
    let apres = gate.status().await?;
    assert!(
        apres.spent_24h.yocto() >= 100_000_000_000_000_000_000_000,
        "la dépense 24 h doit avoir augmenté : {:?}",
        apres.spent_24h
    );

    Ok(())
}

#[tokio::test]
#[ignore = "parle au testnet public : signe de vraies transactions"]
async fn mcp_testnet_ne_signature_pas_un_refus() -> anyhow::Result<()> {
    let gate = gate().await?;
    // Un refus (49 NEAR) ne doit laisser aucune trace de transaction.
    let bloque = handle_tool_call(
        &gate,
        "wallet_propose",
        &transfer("mallory.test.near", "49"),
        Lang::En,
    )
    .await;
    assert!(bloque.contains("VERDICT: BLOCK"), "sortie : {bloque}");
    assert!(!bloque.contains("tx "), "aucune transaction : {bloque}");
    Ok(())
}
