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
pub fn parse_propose_args(args: &Value) -> Result<(Demande, String), String> {
    let obj = args
        .as_object()
        .ok_or_else(|| "les arguments doivent être un objet JSON".to_string())?;
    for key in obj.keys() {
        if !PROPOSE_FIELDS.contains(&key.as_str()) {
            return Err(format!(
                "champ inconnu `{key}` — champs acceptés : {}",
                PROPOSE_FIELDS.join(", ")
            ));
        }
    }
    let intent = obj
        .get("intent")
        .and_then(Value::as_str)
        .ok_or_else(|| "champ `intent` manquant".to_string())?
        .to_string();

    let kind = obj.get("kind").and_then(Value::as_str).unwrap_or("transfer");
    let demand = match kind {
        "transfer" => {
            let to = obj
                .get("to")
                .and_then(Value::as_str)
                .ok_or_else(|| "champ `to` requis pour un transfert".to_string())?;
            let amount_near = obj
                .get("amount_near")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    "champ `amount_near` manquant : montant absent refusé (en NEAR décimal, ex: \"1.5\")"
                        .to_string()
                })?;
            let amount = crate::cli::parse_near(amount_near)
                .map_err(|e| format!("montant invalide `{amount_near}` : {e}"))?;
            Demande::Transfer {
                to: to.to_string(),
                amount,
            }
        }
        "function_call" => {
            let contract = obj
                .get("contract")
                .and_then(Value::as_str)
                .ok_or_else(|| "champ `contract` requis pour un appel de contrat".to_string())?;
            let method = obj
                .get("method")
                .and_then(Value::as_str)
                .ok_or_else(|| "champ `method` requis pour un appel de contrat".to_string())?;
            let deposit = match obj.get("deposit_near").and_then(Value::as_str) {
                Some(text) => crate::cli::parse_near(text)
                    .map_err(|e| format!("dépôt invalide `{text}` : {e}"))?,
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
        other => {
            return Err(format!(
                "kind inconnu `{other}` — attendu `transfer` ou `function_call`"
            ))
        }
    };
    Ok((demand, intent))
}

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

pub async fn handle_tool_call(gate: &Gatekeeper, name: &str, args: &Value) -> String {
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
            let (demand, intent_text) = match parse_propose_args(args) {
                Ok(pair) => pair,
                Err(message) => return format!("REJET (demande mal formée) : {message}"),
            };

            let intent = Intent {
                summary: intent_text,
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
