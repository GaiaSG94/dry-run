//! Serveur MCP (Model Context Protocol) JSON-RPC 2.0 sur `stdin`/`stdout`.
//!
//! Expose nativement Dry Run comme proxy de wallet sécurisé pour n'importe
//! quel agent IA (Claude Desktop, Cursor, ZeroClaw, Codex).
//!
//! Outils :
//! - `wallet_status` : solde, plafonds 24 h, contrats connus, gel
//! - `wallet_propose` : simule une transaction, applique les règles et rend la décision
//! - `wallet_freeze` : déclenche le gel d'urgence
//! - `wallet_explain` : explique en langage naturel pourquoi une règle a mordu
//!
//! Deux règles de protocole sont respectées à la lettre :
//! - une **notification** (message sans `id`) ne reçoit **aucune réponse** ;
//! - la réponse dit **où** la transaction a été exécutée : jamais « oui »
//!   trompeur pour une exécution restée en mémoire.

use std::io::{BufRead, Write};
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent};
use serde_json::{json, Value};

use crate::gate::Gatekeeper;
use crate::lang::Lang;

/// Champs acceptés par l'outil `wallet_propose`.
const PROPOSE_FIELDS: &[&str] = &[
    "kind",
    "to",
    "amount_near",
    "contract",
    "method",
    "args",
    "deposit_near",
    "intent",
];

/// Analyse et valide strictement les arguments de `wallet_propose`.
///
/// Toute demande mal formée est rejetée avant simulation : champ inconnu,
/// `intent` manquant, champ de montant absent ou non numérique. Aucun
/// défaut silencieux : un montant oublié ne doit jamais devenir 1 NEAR.
pub fn parse_propose_args(args: &Value, lang: Lang) -> Result<(Demande, String), String> {
    let obj = args.as_object().ok_or_else(|| match lang {
        Lang::En => "the arguments must be a JSON object".to_string(),
        Lang::Fr => "les arguments doivent être un objet JSON".to_string(),
    })?;
    for key in obj.keys() {
        if !PROPOSE_FIELDS.contains(&key.as_str()) {
            return Err(lang.unknown_field(key, PROPOSE_FIELDS));
        }
    }
    let intent = obj
        .get("intent")
        .and_then(Value::as_str)
        .ok_or_else(|| lang.missing_field("intent"))?
        .to_string();

    let kind = obj
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("transfer");
    let demand = match kind {
        "transfer" => {
            let to = obj
                .get("to")
                .and_then(Value::as_str)
                .ok_or_else(|| lang.missing_field("to"))?;
            let amount_near = obj
                .get("amount_near")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    lang.missing_field_why(
                        "amount_near",
                        match lang {
                            Lang::En => "a missing amount is refused (decimal NEAR, e.g. \"1.5\")",
                            Lang::Fr => "montant absent refusé (en NEAR décimal, ex: \"1.5\")",
                        },
                    )
                })?;
            let amount = crate::cli::parse_near(amount_near)
                .map_err(|_| lang.invalid_amount(amount_near))?;
            Demande::Transfer {
                to: to.to_string(),
                amount,
            }
        }
        "function_call" => {
            let contract = obj
                .get("contract")
                .and_then(Value::as_str)
                .ok_or_else(|| lang.missing_field("contract"))?;
            let method = obj
                .get("method")
                .and_then(Value::as_str)
                .ok_or_else(|| lang.missing_field("method"))?;
            let deposit = match obj.get("deposit_near").and_then(Value::as_str) {
                Some(text) => {
                    crate::cli::parse_near(text).map_err(|_| lang.invalid_amount(text))?
                }
                None => 0,
            };
            Demande::FunctionCall {
                contract: contract.to_string(),
                method: method.to_string(),
                args: obj.get("args").cloned().unwrap_or(json!({})),
                deposit,
                gas: 30_000_000_000_000,
            }
        }
        other => return Err(lang.unknown_kind(other)),
    };
    Ok((demand, intent))
}

/// Boucle principale du serveur MCP sur stdin/stdout.
pub async fn run_mcp_server(gate: Arc<Gatekeeper>, lang: Lang) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let text = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                writeln!(stdout, "{}", serde_json::to_string(&err_resp)?)?;
                stdout.flush()?;
                continue;
            }
        };

        if let Some(response) = handle_request(&gate, &request, lang).await {
            writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
            stdout.flush()?;
        }
    }
    Ok(())
}

/// Traite un message JSON-RPC et rend la réponse, ou `None` pour une notification.
///
/// JSON-RPC 2.0 : un message **sans** `id` est une notification, et une
/// notification ne reçoit **aucune réponse**. Certains clients MCP stricts le
/// signalent comme une erreur de protocole.
pub async fn handle_request(gate: &Gatekeeper, request: &Value, lang: Lang) -> Option<Value> {
    let is_notification = request.get("id").is_none();
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").cloned().unwrap_or(json!({}));

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": { "name": "dryrun-mcp", "version": "0.1.0" },
                "capabilities": { "tools": {} }
            }
        }),
        "notifications/initialized" | "initialized" => {
            // Notification de mise en route du client : on n'y répond pas.
            return None;
        }
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": lang.tools() }
        }),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let content = handle_tool_call(gate, name, &args, lang).await;
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "content": [{ "type": "text", "text": content }] }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": format!("Method not found: {method}") }
        }),
    };

    if is_notification {
        // Une notification reste sans réponse, même si la méthode a été
        // reconnue : c'est le client qui n'attend rien.
        return None;
    }
    Some(response)
}

/// Traite un appel d'outil MCP et rend la réponse textuelle.
pub async fn handle_tool_call(gate: &Gatekeeper, name: &str, args: &Value, lang: Lang) -> String {
    match name {
        "wallet_status" => match gate.status().await {
            Ok(s) => lang.status_text(&s),
            Err(e) => match lang {
                Lang::En => format!("Status error: {e}"),
                Lang::Fr => format!("Erreur statut : {e}"),
            },
        },
        "wallet_freeze" => match gate.freeze().await {
            Ok(_) => lang.frozen_text().to_string(),
            Err(e) => lang.frozen_error(&e.to_string()),
        },
        "wallet_explain" => {
            let rule = args.get("rule_id").and_then(Value::as_str).unwrap_or("");
            lang.explain_rule(rule)
        }
        "wallet_propose" => {
            let (demand, intent_text) = match parse_propose_args(args, lang) {
                Ok(pair) => pair,
                Err(message) => return lang.rejected(&message),
            };

            let intent = Intent {
                summary: intent_text,
                beneficiaries: vec![demand.target().unwrap_or("bob.test.near").to_string()],
                min_in: vec![],
                max_out: None,
            };

            match gate.propose(&demand, &intent).await {
                Ok(outcome) => {
                    let executed = lang.executed_line(
                        outcome.executed,
                        outcome.on_chain,
                        outcome.tx_hash.as_deref(),
                    );
                    let rules = lang.rules_line(&outcome.card_view.rules);
                    format!(
                        "{}\n{}\n{}\n\n{}\n{}",
                        lang.verdict_line(outcome.verdict.as_str()),
                        executed,
                        rules,
                        lang.card_title(),
                        lang.effects_card(&outcome.card_view),
                    )
                }
                Err(e) => lang.simulation_error(&e.to_string()),
            }
        }
        _ => lang.unknown_tool(name),
    }
}
