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

use std::io::{BufRead, Write};
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent};
use serde_json::{json, Value};

use crate::gate::Gatekeeper;

/// Boucle principale du serveur MCP sur stdin/stdout.
pub async fn run_mcp_server(gate: Arc<Gatekeeper>) -> anyhow::Result<()> {
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
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "wallet_status",
                            "description": "Consulter l'état du wallet coffre : soldes, plafonds glissants 24h, contrats approuvés, statut de gel.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "wallet_propose",
                            "description": "Proposer une action financière (transfert ou appel de contrat). Elle est SIMULÉE dans un bac à sable local avant toute signature. Le verdict (ALLOW, WARN, BLOCK) et la carte des effets sont retournés.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "kind": { "type": "string", "enum": ["transfer", "function_call"] },
                                    "to": { "type": "string", "description": "Destinataire du transfert" },
                                    "amount_near": { "type": "string", "description": "Montant en NEAR décimal (ex: '1.5')" },
                                    "contract": { "type": "string", "description": "Contrat visé par l'appel" },
                                    "method": { "type": "string", "description": "Méthode appelée" },
                                    "args": { "type": "object", "description": "Arguments JSON" },
                                    "deposit_near": { "type": "string", "description": "Dépôt joint en NEAR (ex: '2.0')" },
                                    "intent": { "type": "string", "description": "Intention déclarée en langage clair (ex: 'payer 1 NEAR à bob')" }
                                },
                                "required": ["intent"]
                            }
                        },
                        {
                            "name": "wallet_freeze",
                            "description": "Geler immédiatement le coffre d'un geste d'urgence via la clé de gel. Bloque toute sortie de fonds ultérieure.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "wallet_explain",
                            "description": "Obtenir l'explication détaillée d'une règle du pare-feu Dry Run (R1 à R12).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "rule_id": { "type": "string", "description": "Identifiant de la règle (ex: 'R1', 'R7', 'R8')" }
                                },
                                "required": ["rule_id"]
                            }
                        }
                    ]
                }
            }),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                let content = handle_tool_call(&gate, name, &args).await;
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

        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }
    Ok(())
}

async fn handle_tool_call(gate: &Gatekeeper, name: &str, args: &Value) -> String {
    match name {
        "wallet_status" => match gate.status().await {
            Ok(s) => format!(
                "Compte coffre : {}\nSolde actuel  : {} NEAR\nPlafond par tx : {} NEAR\nPlafond 24h   : {} NEAR\nDépensé 24h   : {} NEAR\nStatut        : {}\nContrats connus : {:?}",
                s.account_id,
                crate::Amount(s.balance.yocto()).near(),
                crate::Amount(s.tx_cap.yocto()).near(),
                crate::Amount(s.daily_cap.yocto()).near(),
                crate::Amount(s.spent_24h.yocto()).near(),
                if s.is_frozen { "GELÉ (FROZEN)" } else { "OUVERT (OPEN)" },
                s.known_contracts.keys().collect::<Vec<_>>()
            ),
            Err(e) => format!("Erreur statut : {e}"),
        },
        "wallet_freeze" => match gate.freeze().await {
            Ok(_) => "Le coffre a été gelé d'un geste. Toute transaction ultérieure sera rejetée.".to_string(),
            Err(e) => format!("Erreur gel : {e}"),
        },
        "wallet_explain" => {
            let rule = args.get("rule_id").and_then(Value::as_str).unwrap_or("");
            explain_rule(rule)
        }
        "wallet_propose" => {
            let intent_text = args.get("intent").and_then(Value::as_str).unwrap_or("opération");
            let kind = args.get("kind").and_then(Value::as_str).unwrap_or("transfer");

            let demand = if kind == "function_call" {
                let contract = args.get("contract").and_then(Value::as_str).unwrap_or("good-dex.test.near");
                let method = args.get("method").and_then(Value::as_str).unwrap_or("swap");
                let deposit_near = args.get("deposit_near").and_then(Value::as_str).unwrap_or("0");
                let deposit = crate::cli::parse_near(deposit_near).unwrap_or(0);
                Demande::FunctionCall {
                    contract: contract.to_string(),
                    method: method.to_string(),
                    args: args.get("args").cloned().unwrap_or(json!({})),
                    deposit,
                    gas: 30_000_000_000_000,
                }
            } else {
                let to = args.get("to").and_then(Value::as_str).unwrap_or("bob.test.near");
                let amount_near = args.get("amount_near").and_then(Value::as_str).unwrap_or("1");
                let amount = crate::cli::parse_near(amount_near).unwrap_or(10u128.pow(24));
                Demande::Transfer { to: to.to_string(), amount }
            };

            let intent = Intent {
                summary: intent_text.to_string(),
                beneficiaries: vec![demand.target().unwrap_or("bob.test.near").to_string()],
                min_in: vec![],
                max_out: None,
            };

            match gate.propose(&demand, &intent).await {
                Ok(outcome) => format!(
                    "VERDICT : {}\nExécuté sur chaîne : {}\nRègles en cause : {:?}\n\nCARTE DES EFFETS SIMULÉS :\n{}",
                    outcome.verdict.as_str(),
                    if outcome.executed { "OUI" } else { "NON (rejeté avant signature)" },
                    outcome.rules.iter().map(|r| &r.id).collect::<Vec<_>>(),
                    outcome.card
                ),
                Err(e) => format!("Erreur simulation : {e}"),
            }
        }
        _ => format!("Outil inconnu : {name}"),
    }
}

fn explain_rule(rule_id: &str) -> String {
    for (id, desc) in dryrun_core::RULES {
        if *id == rule_id {
            return format!("Règle {id} : {desc}");
        }
    }
    format!("Règle inconnue : {rule_id}. Les règles valides sont R1 à R12.")
}
