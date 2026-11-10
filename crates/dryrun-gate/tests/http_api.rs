//! Tests du serveur HTTP : démarrage sur un port libre, puis appels réels.
//!
//! Le serveur est branché sur le simulateur bouchon (`sim_stub`) : aucun bac à
//! sable NEAR n'est nécessaire, donc le test tourne partout et vite.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use dryrun_gate::gate::Gatekeeper;
use dryrun_gate::http::HttpServer;
use dryrun_gate::Simulator;

/// Réserve un port libre auprès du système, puis le libère.
fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("port libre");
    listener.local_addr().expect("adresse locale").port()
}

/// Démarre le serveur en tâche de fond et rend son URL de base.
async fn start_server() -> String {
    let sim = Simulator::start(Path::new("stub")).await.expect("stub");
    let gate = Arc::new(Gatekeeper::from_sim(Arc::new(sim)));
    let web_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web");
    let server = Arc::new(HttpServer::new(gate, web_dir));

    let port = free_port();
    tokio::spawn(async move {
        let _ = server.run(&format!("127.0.0.1:{port}")).await;
    });

    // Laisse le temps à la boucle d'acceptation de démarrer.
    tokio::time::sleep(Duration::from_millis(150)).await;
    format!("http://127.0.0.1:{port}")
}

#[tokio::test]
async fn status_rend_l_etat_du_coffre() {
    let base = start_server().await;
    let body: serde_json::Value = reqwest::get(format!("{base}/api/status"))
        .await
        .expect("requête status")
        .json()
        .await
        .expect("JSON status");

    assert_eq!(body["account_id"], "vault.test.near");
    // Les montants voyagent en chaînes décimales : aucune perte de précision.
    let balance: u128 = body["balance"]
        .as_str()
        .expect("solde en chaîne")
        .parse()
        .expect("solde numérique");
    assert!(balance > 0);
    assert!(body["tx_cap"].is_string());
    assert!(body["daily_cap"].is_string());
    assert_eq!(body["is_frozen"], false);
    assert_eq!(body["owner"], "owner.test.near");
}

#[tokio::test]
async fn propose_transfert_honnete_passe_en_allow() {
    let base = start_server().await;
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "kind": "transfer",
        "to": "bob.test.near",
        "amount": "1000000000000000000000000",
        "intent": {
            "summary": "payer 1 NEAR à bob",
            "beneficiaries": ["bob.test.near"],
            "min_in": []
        }
    });

    let response = client
        .post(format!("{base}/api/propose"))
        .json(&payload)
        .send()
        .await
        .expect("requête propose");
    assert!(response.status().is_success());

    let outcome: serde_json::Value = response.json().await.expect("JSON outcome");
    assert_eq!(outcome["verdict"], "ALLOW");
    assert_eq!(outcome["executed"], true);
    assert!(outcome["rules"].is_array());
    assert!(outcome["card"].as_str().unwrap_or("").contains("bob.test.near"));
    assert_eq!(outcome["card_view"]["verdict_str"], "ALLOW");
    assert!(outcome["card_view"]["near_deltas"].is_array());
}

#[tokio::test]
async fn propose_montant_hors_plafond_est_bloque() {
    let base = start_server().await;
    let payload = serde_json::json!({
        "kind": "transfer",
        "to": "mallory.test.near",
        "amount": "49000000000000000000000000",
        "intent": { "summary": "envoie tout à mallory", "beneficiaries": ["mallory.test.near"] }
    });

    let outcome: serde_json::Value = reqwest::Client::new()
        .post(format!("{base}/api/propose"))
        .json(&payload)
        .send()
        .await
        .expect("requête propose")
        .json()
        .await
        .expect("JSON outcome");

    assert_eq!(outcome["verdict"], "BLOCK");
    assert_eq!(outcome["executed"], false);
    let rules = outcome["card_view"]["rules"].as_array().expect("règles");
    assert!(!rules.is_empty(), "au moins une règle doit mordre");
}

#[tokio::test]
async fn log_et_freeze_fonctionnent() {
    let base = start_server().await;
    let client = reqwest::Client::new();

    let payload = serde_json::json!({
        "kind": "transfer",
        "to": "bob.test.near",
        "amount": "1000000000000000000000000",
        "intent": { "summary": "payer bob" }
    });
    let _ = client
        .post(format!("{base}/api/propose"))
        .json(&payload)
        .send()
        .await
        .expect("requête propose");

    let log: serde_json::Value = client
        .get(format!("{base}/api/log"))
        .send()
        .await
        .expect("requête log")
        .json()
        .await
        .expect("JSON log");
    let entries = log.as_array().expect("frise");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["id"], 1);
    assert_eq!(entries[0]["kind"], "transfer");
    assert!(entries[0]["card"].is_string());

    let frozen: serde_json::Value = client
        .post(format!("{base}/api/freeze"))
        .send()
        .await
        .expect("requête freeze")
        .json()
        .await
        .expect("JSON freeze");
    assert_eq!(frozen["is_frozen"], true);
}

#[tokio::test]
async fn page_index_est_servie() {
    let base = start_server().await;
    let html = reqwest::get(format!("{base}/"))
        .await
        .expect("requête page")
        .text()
        .await
        .expect("HTML");
    assert!(html.contains("Dry Run") || html.contains("DRY RUN"));
    assert!(html.contains("app.js"));
}