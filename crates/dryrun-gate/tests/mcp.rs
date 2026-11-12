//! Tests du serveur MCP : rejet strict des demandes mal formées.
//!
//! Scénario de régression : une demande de 49 NEAR envoyée avec un nom de
//! champ erroné ne doit PAS être interprétée comme 1 NEAR et acceptée.

use std::sync::Arc;

use serde_json::json;

use dryrun_gate::gate::Gatekeeper;
use dryrun_gate::mcp::handle_tool_call;
use dryrun_gate::Simulator;

/// Construit un garde-barrière branché sur le simulateur bouchon.
async fn gate() -> Gatekeeper {
    let sim = Simulator::stub().await.expect("simulateur bouchon");
    Gatekeeper::from_sim(Arc::new(sim))
}

#[tokio::test]
async fn champ_mal_nomme_est_rejete_meme_avec_49_near() {
    let gate = gate().await;
    // Le montant voyage dans `amount` au lieu de `amount_near` : le champ
    // est inconnu, la demande est rejetée — jamais lue comme 1 NEAR.
    let args = json!({
        "intent": "payer 49 NEAR à mallory",
        "kind": "transfer",
        "to": "mallory.test.near",
        "amount": "49000000000000000000000000"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("REJET"), "sortie : {out}");
    assert!(out.contains("champ inconnu `amount`"), "sortie : {out}");
    assert!(!out.contains("ALLOW"), "sortie : {out}");
}

#[tokio::test]
async fn montant_absent_est_rejete() {
    let gate = gate().await;
    let args = json!({
        "intent": "payer à bob",
        "kind": "transfer",
        "to": "bob.test.near"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("REJET"), "sortie : {out}");
    assert!(out.contains("amount_near"), "sortie : {out}");
    assert!(!out.contains("ALLOW"), "sortie : {out}");
}

#[tokio::test]
async fn montant_non_numerique_est_rejete() {
    let gate = gate().await;
    let args = json!({
        "intent": "payer 49 NEAR à bob",
        "kind": "transfer",
        "to": "bob.test.near",
        "amount_near": "quarante-neuf"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("REJET"), "sortie : {out}");
    assert!(out.contains("montant invalide"), "sortie : {out}");
    assert!(!out.contains("ALLOW"), "sortie : {out}");
}

#[tokio::test]
async fn intent_absent_est_rejete() {
    let gate = gate().await;
    let args = json!({
        "kind": "transfer",
        "to": "bob.test.near",
        "amount_near": "1"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("REJET"), "sortie : {out}");
    assert!(out.contains("`intent` manquant"), "sortie : {out}");
}

#[tokio::test]
async fn kind_inconnu_est_rejete() {
    let gate = gate().await;
    let args = json!({
        "intent": "virement mystère",
        "kind": "stake",
        "to": "bob.test.near",
        "amount_near": "1"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("REJET"), "sortie : {out}");
    assert!(out.contains("kind inconnu"), "sortie : {out}");
}

#[tokio::test]
async fn demande_bien_formee_passe_normalement() {
    let gate = gate().await;
    let args = json!({
        "intent": "payer 1 NEAR à bob",
        "kind": "transfer",
        "to": "bob.test.near",
        "amount_near": "1"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args).await;
    assert!(out.contains("ALLOW"), "sortie : {out}");
    assert!(out.contains("bob.test.near"), "sortie : {out}");
}
