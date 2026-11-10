//! Client JSON-RPC minimal pour un bac à sable NEAR.
//!
//! On n'utilise volontairement pas `near-api` : construire et signer soi-même
//! avec `near-primitives` + `near-crypto` évite une couche d'abstraction
//! (et une version d'`AccountId` différente de `near-sandbox`).

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use near_crypto::{PublicKey, SecretKey, Signature};
use near_token::NearToken;
use serde_json::{json, Value};

use near_primitives::hash::CryptoHash;
use near_primitives::types::AccountId;
use near_primitives::views::{
    AccessKeyInfoView, AccessKeyView, AccountView, FinalExecutionOutcomeView, FinalExecutionStatus,
};

/// Une exécution est terminée quand elle n'est plus « pas commencée » ou « en cours ».
pub fn status_done(status: &FinalExecutionStatus) -> bool {
    !matches!(
        status,
        FinalExecutionStatus::NotStarted | FinalExecutionStatus::Started
    )
}

/// Encodage base64 standard (utilisé partout dans les RPC NEAR).
pub fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Décode un champ base64 renvoyé par un nœud.
pub fn unb64(text: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .with_context(|| "base64 invalide dans la réponse RPC")
}

/// Un compte tel que lu sur une chaîne (vue RPC).
#[derive(Clone, Debug)]
pub struct AccountInfo {
    pub amount: NearToken,
    pub code_hash: String,
    pub storage_usage: u64,
}

impl AccountInfo {
    /// Le compte porte-t-il du code ?
    pub fn has_code(&self) -> bool {
        self.code_hash != "11111111111111111111111111111111"
    }
}

/// Un point d'entrée JSON-RPC NEAR.
pub struct Rpc {
    url: String,
    http: reqwest::Client,
}

impl Rpc {
    /// Construit un client à partir d'une adresse de type `127.0.0.1:3030` ou `http://...`.
    pub fn new(addr: &str) -> Result<Self> {
        let url = if addr.starts_with("http") {
            addr.trim_end_matches('/').to_string()
        } else {
            format!("http://{}", addr.trim_end_matches('/'))
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("client HTTP RPC")?;
        Ok(Rpc { url, http })
    }

    /// URL RPC sous sa forme `http://hôte:port`.
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
            bail!("RPC {method} : {}", err);
        }
        payload
            .get("result")
            .cloned()
            .ok_or_else(|| anyhow!("RPC {method} : réponse sans champ `result`"))
    }

    /// Appel JSON-RPC dont l'erreur est attendue (compte absent, méthode inconnue).
    async fn try_call(&self, method: &str, params: Value) -> Option<Value> {
        self.call(method, params).await.ok()
    }

    /// Hauteur du dernier bloc finalisé.
    pub async fn height(&self) -> Result<u64> {
        let result = self.call("status", json!({})).await?;
        result
            .pointer("/sync_info/latest_block_height")
            .and_then(Value::as_u64)
            .ok_or_else(|| anyhow!("status : hauteur de bloc absente"))
    }

    /// Hash du dernier bloc finalisé, nécessaire pour signer.
    pub async fn block_hash(&self) -> Result<CryptoHash> {
        let result = self.call("block", json!({"finality": "final"})).await?;
        let hash = result
            .pointer("/header/hash")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("block : hash absent"))?;
        hash.parse().context("block : hash invalide")
    }

    /// État d'un compte, `None` s'il n'existe pas.
    pub async fn account(&self, account: &AccountId) -> Result<Option<AccountInfo>> {
        let result = self
            .try_call(
                "query",
                json!({"request_type": "view_account", "finality": "optimistic", "account_id": account}),
            )
            .await;
        let Some(result) = result else { return Ok(None) };
        let view: AccountView = serde_json::from_value(result).context("view_account illisible")?;
        Ok(Some(AccountInfo {
            amount: view.amount,
            code_hash: view.code_hash.to_string(),
            storage_usage: view.storage_usage,
        }))
    }

    /// Clés d'accès d'un compte.
    pub async fn access_keys(&self, account: &AccountId) -> Result<Vec<AccessKeyInfoView>> {
        let Some(result) = self
            .try_call(
                "query",
                json!({"request_type": "view_access_key_list", "finality": "optimistic", "account_id": account}),
            )
            .await
        else {
            return Ok(Vec::new());
        };
        let keys = result.get("keys").cloned().unwrap_or_else(|| json!([]));
        serde_json::from_value(keys).context("view_access_key_list illisible")
    }

    /// Nonce courant d'une clé, `None` si la clé n'existe pas.
    pub async fn key_nonce(&self, account: &AccountId, key: &PublicKey) -> Result<Option<u64>> {
        let result = self
            .try_call(
                "query",
                json!({
                    "request_type": "view_access_key",
                    "finality": "optimistic",
                    "account_id": account,
                    "public_key": key.to_string(),
                }),
            )
            .await;
        let Some(result) = result else { return Ok(None) };
        let view: AccessKeyView = serde_json::from_value(result).context("view_access_key illisible")?;
        Ok(Some(view.nonce))
    }

    /// Octets du contrat déployé sur un compte.
    pub async fn code(&self, account: &AccountId) -> Result<Option<Vec<u8>>> {
        let Some(result) = self
            .try_call(
                "query",
                json!({"request_type": "view_code", "finality": "optimistic", "account_id": account}),
            )
            .await
        else {
            return Ok(None);
        };
        let code = match result.get("code_base64").and_then(Value::as_str) {
            Some(text) => unb64(text)?,
            None => return Ok(None),
        };
        Ok(Some(code))
    }

    /// Appel `view` : rend la valeur JSON renvoyée par le contrat.
    pub async fn view_call(
        &self,
        account: &AccountId,
        method: &str,
        args: &Value,
    ) -> Result<Value> {
        let args = serde_json::to_vec(args).context("sérialisation des arguments")?;
        let result = self
            .call(
                "query",
                json!({
                    "request_type": "call",
                    "finality": "optimistic",
                    "account_id": account,
                    "method_name": method,
                    "args_base64": b64(&args),
                }),
            )
            .await?;
        let bytes: Vec<u8> = result
            .get("result")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(|v| v.as_u64()).map(|v| v as u8).collect())
            .ok_or_else(|| anyhow!("call {account}.{method} : champ `result` absent"))?;
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        serde_json::from_slice(&bytes)
            .with_context(|| format!("call {account}.{method} : sortie JSON invalide"))
    }

    /// Envoie une transaction signée et attend la fin des reçus.
    pub async fn send(&self, tx: &near_primitives::transaction::SignedTransaction) -> Result<FinalExecutionOutcomeView> {
        let bytes = borsh::to_vec(tx).context("encodage borsh de la transaction")?;
        let params = json!({
            "signed_tx_base64": b64(&bytes),
            "wait_until": "EXECUTED_OPTIMISTIC",
        });
        let result = match self.call("send_tx", params.clone()).await {
            Ok(result) => result,
            // Les vieux nœuds n'ont que `broadcast_tx_commit`.
            Err(_) => {
                self.call("broadcast_tx_commit", json!([b64(&bytes)]))
                    .await
                    .context("envoi de la transaction refusé")?
            }
        };
        let outcome: FinalExecutionOutcomeView =
            serde_json::from_value(result).context("résultat d'exécution illisible")?;
        self.wait_receipts(outcome).await
    }

    /// Relit une transaction jusqu'à ce que tous les reçus soient exécutés.
    async fn wait_receipts(
        &self,
        outcome: FinalExecutionOutcomeView,
    ) -> Result<FinalExecutionOutcomeView> {
        if status_done(&outcome.status) {
            return Ok(outcome);
        }
        let sender = outcome.transaction.signer_id.clone();
        let hash = outcome.transaction.hash.to_string();
        let deadline = std::time::Instant::now() + Duration::from_secs(30);
        let mut last = outcome;
        while std::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let result = self
                .call(
                    "tx",
                    json!({"tx_hash": hash, "sender_account_id": sender, "wait_until": "EXECUTED_OPTIMISTIC"}),
                )
                .await?;
            last = serde_json::from_value::<FinalExecutionOutcomeView>(result)
                .context("suivi de transaction illisible")?;
            if status_done(&last.status) {
                return Ok(last);
            }
        }
        Ok(last)
    }
}

/// Construit et signe une transaction (une seule action par appel ici).
pub fn sign_transaction(
    signer: &SecretKey,
    signer_id: &AccountId,
    receiver_id: &AccountId,
    nonce: u64,
    block_hash: CryptoHash,
    actions: Vec<near_primitives::transaction::Action>,
) -> near_primitives::transaction::SignedTransaction {
    use near_primitives::transaction::{Transaction, TransactionV0, SignedTransaction};

    let transaction = Transaction::V0(TransactionV0 {
        signer_id: signer_id.clone(),
        public_key: signer.public_key(),
        nonce,
        receiver_id: receiver_id.clone(),
        block_hash,
        actions,
    });
    let (hash, _) = transaction.get_hash_and_size();
    let signature: Signature = signer.sign(hash.as_ref());
    SignedTransaction::new(signature, transaction)
}
