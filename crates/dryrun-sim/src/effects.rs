//! Capture d'état avant/après une simulation et fabrication des `Effects`.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use near_primitives::types::AccountId;
use serde_json::Value;

use dryrun_core::{AccountSnap, CodeEvent, Effects, FtDelta, KeyEvent, NearDelta, NftEvent, ReceiptInfo, Snapshot};

use crate::accounts;
use crate::rpc::{b64, unb64, Rpc};

/// Coût de stockage par octet (yoctoNEAR), valeur du protocole NEAR.
pub const STORAGE_PRICE_PER_BYTE: u128 = 10u128.pow(19);

/// Une clé d'accès décrite de façon stable et lisible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyDesc {
    pub public_key: String,
    pub scope: String,
}

/// État d'un compte observé dans le bac à sable de simulation.
#[derive(Clone, Debug, Default)]
pub struct AccountSnapshot {
    pub account: String,
    pub balance: u128,
    pub code_hash: String,
    pub storage_usage: u64,
    pub keys: Vec<KeyDesc>,
    pub ft: Vec<(String, u128)>,
}

impl AccountSnapshot {
    /// Clés triées, pour comparer deux états sans tenir compte de l'ordre.
    pub fn key_set(&self) -> BTreeMap<String, String> {
        self.keys
            .iter()
            .map(|key| (key.public_key.clone(), key.scope.clone()))
            .collect()
    }

    /// Solde d'un jeton fongible, zéro s'il n'y en a pas.
    pub fn ft_of(&self, token: &str) -> u128 {
        self.ft
            .iter()
            .find(|(name, _)| name == token)
            .map(|(_, amount)| *amount)
            .unwrap_or(0)
    }
}

/// Un état complet observé à un instant donné.
#[derive(Clone, Debug, Default)]
pub struct SnapshotSet {
    pub height: u64,
    pub block_hash: String,
    pub accounts: Vec<AccountSnapshot>,
}

impl SnapshotSet {
    /// Le compte cherché, s'il a été observé.
    pub fn get(&self, account: &str) -> Option<&AccountSnapshot> {
        self.accounts.iter().find(|snap| snap.account == account)
    }
}

/// Résultat d'exécution servant à remplir les champs non étatiques.
#[derive(Clone, Debug, Default)]
pub struct ExecutionInfo {
    pub receipts: Vec<ReceiptInfo>,
    pub gas_burnt: u64,
    pub height: u64,
    pub block_hash: String,
}

/// Lit l'état des comptes surveillés dans un bac à sable.
pub async fn snapshot(
    rpc: &Rpc,
    watched: &[String],
    token: Option<&str>,
) -> Result<SnapshotSet> {
    let height = rpc.height().await?;
    let block_hash = rpc.block_hash().await?.to_string();
    let mut accounts = Vec::with_capacity(watched.len());
    for name in watched {
        accounts.push(account_snapshot(rpc, name, token).await?);
    }
    Ok(SnapshotSet { height, block_hash, accounts })
}

/// Lit un compte : solde, code, clés, jetons.
async fn account_snapshot(rpc: &Rpc, name: &str, token: Option<&str>) -> Result<AccountSnapshot> {
    let account: AccountId = name.parse().with_context(|| format!("compte invalide : {name}"))?;
    let info = rpc.account(&account).await?;
    let keys = rpc
        .access_keys(&account)
        .await?
        .iter()
        .map(|key| KeyDesc {
            public_key: key.public_key.to_string(),
            scope: crate::chain::scope_of_public(&key.access_key.permission),
        })
        .collect();
    let ft = match (token, info.as_ref()) {
        (Some(token), Some(_)) => ft_balance(rpc, token, name).await?,
        _ => 0,
    };
    Ok(AccountSnapshot {
        account: name.to_string(),
        balance: info.as_ref().map(|i| i.amount.as_yoctonear()).unwrap_or(0),
        code_hash: info.as_ref().map(|i| i.code_hash.clone()).unwrap_or_default(),
        storage_usage: info.as_ref().map(|i| i.storage_usage).unwrap_or(0),
        keys,
        ft: match token {
            Some(token) => vec![(token.to_string(), ft)],
            None => Vec::new(),
        },
    })
}

/// Solde d'un jeton fongible, zéro si le contrat ne répond pas.
async fn ft_balance(rpc: &Rpc, token: &str, account: &str) -> Result<u128> {
    let token_id: AccountId = token.parse()?;
    let value: Value = match rpc
        .view_call(&token_id, "ft_balance_of", &serde_json::json!({"account_id": account}))
        .await
    {
        Ok(value) => value,
        Err(_) => return Ok(0),
    };
    Ok(crate::chain::amount_of(&value).unwrap_or(0))
}

/// Construit les `Effects` à partir des deux états et du résultat d'exécution.
pub fn effects(
    before: &SnapshotSet,
    after: &SnapshotSet,
    exec: &ExecutionInfo,
    vault: &str,
    ignored_keys: &[String],
) -> Effects {
    Effects {
        snapshot: build_snapshot(after),
        near_delta: near_deltas(before, after),
        ft_delta: ft_deltas(before, after),
        nft: nft_events(&exec.receipts),
        keys: key_events(before, after, ignored_keys),
        code: code_events(before, after),
        receipts: exec.receipts.clone(),
        stranded: is_stranded(before, after, &exec.receipts),
        gas_burnt: exec.gas_burnt,
        storage_delta: storage_delta(before, after, vault),
    }
}

/// Instantané lisible de l'état final.
fn build_snapshot(after: &SnapshotSet) -> Snapshot {
    Snapshot {
        height: after.height,
        block_hash: after.block_hash.clone(),
        accounts: after
            .accounts
            .iter()
            .map(|snap| AccountSnap {
                id: snap.account.clone(),
                code_hash: snap.code_hash.clone(),
            })
            .collect(),
    }
}

/// Variations de solde NEAR, frais de gas compris.
fn near_deltas(before: &SnapshotSet, after: &SnapshotSet) -> Vec<NearDelta> {
    after
        .accounts
        .iter()
        .map(|snap| {
            let previous = before.get(&snap.account).map(|old| old.balance).unwrap_or(0);
            NearDelta {
                account: snap.account.clone(),
                delta: snap.balance as i128 - previous as i128,
            }
        })
        .collect()
}

/// Variations des soldes de jetons fongibles connus.
fn ft_deltas(before: &SnapshotSet, after: &SnapshotSet) -> Vec<FtDelta> {
    let mut out = Vec::new();
    for snap in &after.accounts {
        for (token, amount) in &snap.ft {
            let previous = before
                .get(&snap.account)
                .map(|old| old.ft_of(token))
                .unwrap_or(0);
            if *amount != previous || previous != 0 {
                out.push(FtDelta {
                    token: token.clone(),
                    account: snap.account.clone(),
                    delta: *amount as i128 - previous as i128,
                });
            }
        }
    }
    out
}

/// Clés apparues ou disparues (`ignored_keys` : clé jetable du simulateur).
fn key_events(before: &SnapshotSet, after: &SnapshotSet, ignored: &[String]) -> Vec<KeyEvent> {
    let mut out = Vec::new();
    for snap in &after.accounts {
        let empty = BTreeMap::new();
        let birth = before.get(&snap.account).map(|old| old.key_set()).unwrap_or(empty);
        let death = snap.key_set();
        for (key, scope) in &death {
            if ignored.contains(key) || birth.contains_key(key) {
                continue;
            }
            out.push(KeyEvent::Added { account: snap.account.clone(), scope: scope.clone() });
        }
        for (key, scope) in &birth {
            if ignored.contains(key) || death.contains_key(key) {
                continue;
            }
            out.push(KeyEvent::Removed { account: snap.account.clone(), scope: scope.clone() });
        }
    }
    out
}

/// Changements de code : première apparition ou remplacement.
fn code_events(before: &SnapshotSet, after: &SnapshotSet) -> Vec<CodeEvent> {
    let mut out = Vec::new();
    for snap in &after.accounts {
        let previous = before.get(&snap.account).map(|old| old.code_hash.clone()).unwrap_or_default();
        if snap.code_hash == previous || snap.code_hash.is_empty() {
            continue;
        }
        if previous.is_empty() {
            out.push(CodeEvent::Deployed { account: snap.account.clone() });
        } else {
            out.push(CodeEvent::Changed { account: snap.account.clone() });
        }
    }
    out
}

/// Vrai si un reçu a échoué alors que des fonds étaient déjà sortis.
fn is_stranded(before: &SnapshotSet, after: &SnapshotSet, receipts: &[ReceiptInfo]) -> bool {
    let failure = receipts.iter().any(|receipt| receipt.status == "failed");
    if !failure {
        return false;
    }
    let outflow = near_deltas(before, after).iter().any(|delta| delta.delta < 0)
        || ft_deltas(before, after).iter().any(|delta| delta.delta < 0);
    outflow
}

/// Coût du stockage gagné par le coffre pendant la transaction, en yoctoNEAR.
fn storage_delta(before: &SnapshotSet, after: &SnapshotSet, vault: &str) -> u128 {
    let previous = before.get(vault).map(|snap| snap.storage_usage).unwrap_or(0);
    let current = after.get(vault).map(|snap| snap.storage_usage).unwrap_or(0);
    current.saturating_sub(previous) as u128 * STORAGE_PRICE_PER_BYTE
}

/// Extrait les événements NFT des journaux de reçus.
pub fn nft_events(receipts: &[ReceiptInfo]) -> Vec<NftEvent> {
    let mut out = Vec::new();
    for receipt in receipts {
        for log in &receipt.logs {
            out.extend(events_in_log(log, &receipt.receiver));
        }
    }
    out
}

/// Analyse un journal : NEP-297 (`EVENT_JSON:`) ou message libre.
fn events_in_log(log: &str, collection: &str) -> Vec<NftEvent> {
    if let Some(rest) = log.strip_prefix("EVENT_JSON:") {
        return nep297(rest, collection);
    }
    let token = field_after(log, "token_id");
    let to = field_after(log, "to").or_else(|| field_after(log, "new_owner_id"));
    let mut out = Vec::new();
    if log.contains("nft_approve") {
        out.push(NftEvent::Approve {
            collection: collection.to_string(),
            token: token.clone(),
            to: to.clone(),
        });
    } else if log.contains("nft_transfer") {
        out.push(NftEvent::Transfer {
            collection: collection.to_string(),
            token,
            to,
        });
    }
    out
}

/// Analyse un événement NEP-297 `nep171`.
fn nep297(json: &str, collection: &str) -> Vec<NftEvent> {
    let Ok(value) = serde_json::from_str::<Value>(json) else {
        return Vec::new();
    };
    if value.get("standard").and_then(Value::as_str) != Some("nep171") {
        return Vec::new();
    }
    let event = value.get("event").and_then(Value::as_str).unwrap_or_default();
    let mut out = Vec::new();
    let data = value.get("data").and_then(Value::as_array).cloned().unwrap_or_default();
    for item in data {
        let token = item
            .get("token_ids")
            .and_then(Value::as_array)
            .and_then(|ids| ids.first())
            .and_then(Value::as_str)
            .unwrap_or_default();
        let to = item
            .get("new_owner_id")
            .or_else(|| item.get("owner_id"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event {
            "nft_transfer" => out.push(NftEvent::Transfer {
                collection: collection.to_string(),
                token: token.to_string(),
                to: to.to_string(),
            }),
            "nft_approve" => out.push(NftEvent::Approve {
                collection: collection.to_string(),
                token: token.to_string(),
                to: to.to_string(),
            }),
            _ => {}
        }
    }
    out
}

/// Cherche `clé: "valeur"` ou `clé:valeur` dans un journal libre.
fn field_after(log: &str, key: &str) -> Option<String> {
    let start = log.find(key)? + key.len();
    let rest = log[start..].trim_start_matches([':', '=', ' ', '"']);
    let value: String = rest
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
        .collect();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

/// Tokens de la démo qu'on relit avant/après (sert à alimenter `ft_delta`).
pub fn watched_token() -> &'static str {
    accounts::TEST_TOKEN
}

/// Encode les octets d'un reçu sous forme lisible (utile aux tests et au débogage).
pub fn log_blob(bytes: &[u8]) -> String {
    b64(bytes)
}

/// Décode un champ base64 d'une réponse RPC.
pub fn decode_blob(text: &str) -> Result<Vec<u8>> {
    unb64(text)
}
