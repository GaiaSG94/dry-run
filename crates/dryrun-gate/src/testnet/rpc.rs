//! Client JSON-RPC minimal pour une chaîne NEAR publique (testnet).
//!
//! Deux lectures suffisent pour brancher le pare-feu sur un coffre déployé :
//! l'état du coffre (`status()`) et le solde du compte (`view_account`).
//! Aucune écriture : la signature passe par la CLI NEAR, qui lit la clé dans
//! le trousseau du système.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use serde_json::{json, Value};

/// URL RPC du testnet public.
pub const TESTNET_RPC_URL: &str = "https://rpc.testnet.near.org";

/// Client JSON-RPC : une vue de contrat et un solde de compte.
pub struct RpcClient {
    url: String,
    http: reqwest::Client,
}

impl RpcClient {
    /// Construit un client pour l'URL donnée.
    pub fn new(url: &str) -> Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("client HTTP RPC")?;
        Ok(Self {
            url: url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// URL du nœud appelé.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Appel JSON-RPC brut : rend `result` ou une erreur lisible.
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc": "2.0", "id": "dryrun", "method": method, "params": params});
        let response = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("RPC {method} : envoi vers {}", self.url))?;
        let status = response.status();
        let payload: Value = response
            .json()
            .await
            .with_context(|| format!("RPC {method} : réponse illisible (HTTP {status})"))?;
        if let Some(err) = payload.get("error") {
            bail!("RPC {method} : {err}");
        }
        payload
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("RPC {method} : réponse sans champ `result`"))
    }

    /// Appel d'une vue de contrat : rend la valeur JSON renvoyée.
    pub async fn view_call(&self, account: &str, method: &str, args: &Value) -> Result<Value> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(serde_json::to_vec(args)?);
        let result = self
            .call(
                "query",
                json!({
                    "request_type": "call_function",
                    "finality": "optimistic",
                    "account_id": account,
                    "method_name": method,
                    "args_base64": encoded,
                }),
            )
            .await?;
        let bytes: Vec<u8> = result
            .get("result")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_u64)
                    .map(|value| value as u8)
                    .collect()
            })
            .ok_or_else(|| anyhow!("call {account}.{method} : champ `result` absent"))?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes)
            .with_context(|| format!("call {account}.{method} : sortie JSON invalide"))
    }

    /// Solde d'un compte, en yoctoNEAR.
    pub async fn account_amount(&self, account: &str) -> Result<u128> {
        let result = self
            .call(
                "query",
                json!({
                    "request_type": "view_account",
                    "finality": "optimistic",
                    "account_id": account,
                }),
            )
            .await?;
        let amount = result
            .get("amount")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("view_account {account} : solde absent"))?;
        amount
            .parse()
            .with_context(|| format!("view_account {account} : solde illisible"))
    }

    /// Hauteur du dernier bloc finalisé.
    pub async fn block_height(&self) -> Result<u64> {
        let result = self.call("status", json!({})).await?;
        result
            .pointer("/sync_info/latest_block_height")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("status : hauteur de bloc absente"))
    }
}
