//! Tests du serveur MCP : rejet strict des demandes mal formées, honnêteté de
//! la ligne « exécuté sur », langue et notifications.
//!
//! Scénario de régression : une demande de 49 NEAR envoyée avec un nom de
//! champ erroné ne doit PAS être interprétée comme 1 NEAR et acceptée.

use std::sync::Arc;

use serde_json::json;

use dryrun_gate::gate::Gatekeeper;
use dryrun_gate::lang::Lang;
use dryrun_gate::mcp::{handle_request, handle_tool_call};
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
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
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
    assert!(out.contains("ALLOW"), "sortie : {out}");
    assert!(out.contains("bob.test.near"), "sortie : {out}");
}

/// Point 1 de la vérification MCP : en mode local, la réponse ne doit pas
/// laisser croire que la transaction est partie sur une vraie chaîne.
#[tokio::test]
async fn reponse_locale_dit_quil_n_y_a_pas_de_chaine() {
    let gate = gate().await;
    let args = json!({
        "intent": "payer 1 NEAR à bob",
        "kind": "transfer",
        "to": "bob.test.near",
        "amount_near": "1"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::Fr).await;
    assert!(
        out.contains("coffre local (simulation)"),
        "la réponse doit nommer le coffre local : {out}"
    );
    assert!(
        !out.contains("Exécuté sur chaîne : OUI"),
        "la réponse ne doit pas prétendre qu'une chaîne a été utilisée : {out}"
    );
    assert!(!out.contains("NEAR testnet"), "sortie : {out}");
}

#[tokio::test]
async fn reponse_anglaise_est_disponible() {
    let gate = gate().await;
    let args = json!({
        "intent": "pay 1 NEAR to bob",
        "kind": "transfer",
        "to": "bob.test.near",
        "amount_near": "1"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::En).await;
    assert!(out.contains("VERDICT: ALLOW"), "sortie : {out}");
    assert!(
        out.contains("Executed on: the local vault (simulation)"),
        "sortie : {out}"
    );
    assert!(out.contains("Rules triggered: none"), "sortie : {out}");
    assert!(out.contains("EFFECTS CARD"), "sortie : {out}");
    assert!(!out.contains("Exécuté"), "sortie : {out}");
}

#[tokio::test]
async fn rejet_en_anglais_est_traduit() {
    let gate = gate().await;
    let args = json!({
        "intent": "pay 1 NEAR to bob",
        "kind": "transfer",
        "to": "bob.test.near",
        "amount": "1000000000000000000000000"
    });
    let out = handle_tool_call(&gate, "wallet_propose", &args, Lang::En).await;
    assert!(out.contains("REJECTED"), "sortie : {out}");
    assert!(out.contains("unknown field `amount`"), "sortie : {out}");
}

#[tokio::test]
async fn tools_list_decrit_les_outils_en_anglais() {
    let tools = Lang::En.tools();
    let names: Vec<String> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        names,
        vec![
            "wallet_status",
            "wallet_propose",
            "wallet_freeze",
            "wallet_explain"
        ]
    );
    let propose = &tools[1];
    assert!(
        propose["description"]
            .as_str()
            .unwrap_or_default()
            .contains("SIMULATED"),
        "la description doit dire que l'action est simulée"
    );
}

#[tokio::test]
async fn wallet_explain_traduit_les_regles() {
    let gate = gate().await;
    let out = handle_tool_call(&gate, "wallet_explain", &json!({"rule_id": "R1"}), Lang::En).await;
    assert!(out.contains("Rule R1"), "sortie : {out}");
    let fr = handle_tool_call(&gate, "wallet_explain", &json!({"rule_id": "R1"}), Lang::Fr).await;
    assert!(fr.contains("Règle R1"), "sortie : {fr}");
}

#[tokio::test]
async fn wallet_status_annonce_le_backend() {
    let gate = gate().await;
    let out = handle_tool_call(&gate, "wallet_status", &json!({}), Lang::En).await;
    assert!(out.contains("Vault account"), "sortie : {out}");
    let fr = handle_tool_call(&gate, "wallet_status", &json!({}), Lang::Fr).await;
    assert!(fr.contains("Compte coffre"), "sortie : {fr}");
}

/// Point 3 de la vérification MCP : une notification (sans `id`) ne reçoit
/// aucune réponse — ni pour `notifications/initialized`, ni pour les autres.
#[tokio::test]
async fn notification_initialized_ne_recoit_aucune_reponse() {
    let gate = gate().await;
    let request = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    });
    let response = handle_request(&gate, &request, Lang::En).await;
    assert!(
        response.is_none(),
        "une notification ne doit pas recevoir de réponse"
    );
}

#[tokio::test]
async fn notification_sans_id_ne_recoit_aucune_reponse() {
    let gate = gate().await;
    // Même `tools/list`, sans `id`, est une notification : aucune réponse.
    let request = json!({ "jsonrpc": "2.0", "method": "tools/list" });
    let response = handle_request(&gate, &request, Lang::En).await;
    assert!(
        response.is_none(),
        "un message sans `id` est une notification"
    );
}

#[tokio::test]
async fn requete_avec_id_recoit_une_reponse() {
    let gate = gate().await;
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });
    let response = handle_request(&gate, &request, Lang::En).await;
    let response = response.expect("une requête avec `id` doit recevoir une réponse");
    assert_eq!(response["id"], 1);
    assert_eq!(response["result"]["serverInfo"]["name"], "dryrun-mcp");
}
