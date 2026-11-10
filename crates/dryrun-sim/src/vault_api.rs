//! Encodage des actions du coffre et lecture de ses vues.
//!
//! Le contrat `vault` (écrit par un autre agent) sérialise ses actions en JSON
//! avec l'étiquetage externe par défaut de serde. On prévoit donc deux formes
//! (`Transfer`/`Call` puis `transfer`/`call`) et deux formes pour le `gas`
//! (chaîne, comme `near_sdk::Gas`, puis nombre).

use anyhow::{Context, Result};
use serde_json::{json, Value};

use dryrun_core::Demande;

use crate::accounts;
use crate::chain::Chain;
use crate::util::amount_of;

/// État du coffre, tel que renvoyé par la vue `status()`.
#[derive(Clone, Debug, Default)]
pub struct VaultStatus {
    /// Réponse brute du contrat, conservée telle quelle.
    pub raw: Value,
}

impl VaultStatus {
    /// Champ brut de la réponse.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.raw.get(key)
    }

    /// Le coffre est-il gelé ?
    pub fn frozen(&self) -> bool {
        self.get("frozen").and_then(Value::as_bool).unwrap_or(false)
    }

    /// Propriétaire déclaré par le coffre.
    pub fn owner(&self) -> Option<String> {
        self.get("owner")
            .and_then(Value::as_str)
            .map(str::to_string)
    }
}

impl serde::Serialize for VaultStatus {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}

/// Encode une demande en actions `VaultAction` candidates (voir l'entête).
pub fn action_variants(demand: &Demande) -> Vec<Value> {
    match demand {
        Demande::Transfer { to, amount } => {
            let payload = json!({"receiver_id": to, "amount": amount.to_string()});
            vec![json!({"Transfer": payload}), json!({"transfer": payload})]
        }
        Demande::FunctionCall {
            contract,
            method,
            args,
            deposit,
            gas,
        } => {
            let encoded = serde_json::to_string(args).unwrap_or_else(|_| "null".to_string());
            let base = json!({
                "receiver_id": contract,
                "method_name": method,
                "args": encoded,
                "deposit": deposit.to_string(),
            });
            vec![
                json!({"Call": with_gas(&base, &gas.to_string())}),
                json!({"Call": with_gas(&base, gas)}),
                json!({"call": with_gas(&base, &gas.to_string())}),
            ]
        }
    }
}

/// Arguments de l'appel `execute(action: VaultAction)`, dans toutes les formes
/// tolérées (near-sdk attend un objet `{"action": ...}`).
pub fn execute_args(demand: &Demande) -> Vec<Value> {
    action_variants(demand)
        .into_iter()
        .map(|action| json!({ "action": action }))
        .collect()
}

/// Copie la charge utile en y ajoutant le champ `gas`.
fn with_gas(base: &Value, gas: &impl serde::Serialize) -> Value {
    let mut payload = base.clone();
    if let Some(object) = payload.as_object_mut() {
        object.insert("gas".to_string(), json!(gas));
    }
    payload
}

/// Lit `status()` sur le coffre.
pub async fn status(chain: &Chain) -> Result<VaultStatus> {
    let raw = chain
        .view_call(accounts::VAULT, "status", &json!({}))
        .await
        .context("lecture de status() sur le coffre")?;
    Ok(VaultStatus { raw })
}

/// Lit les plafonds `(par transaction, sur 24 h)`.
pub async fn caps(chain: &Chain) -> Result<(u128, u128)> {
    let value = chain
        .view_call(accounts::VAULT, "caps", &json!({}))
        .await
        .context("lecture de caps() sur le coffre")?;
    Ok((
        u128_field(&value, "tx_cap"),
        u128_field(&value, "daily_cap"),
    ))
}

/// Lit la dépense des dernières 24 heures.
pub async fn spent_24h(chain: &Chain) -> Result<u128> {
    let value = chain
        .view_call(accounts::VAULT, "spent_24h", &json!({}))
        .await
        .context("lecture de spent_24h() sur le coffre")?;
    Ok(amount_of(&value)
        .or_else(|| u128_field_opt(&value, "spent_24h"))
        .unwrap_or(0))
}

/// Lit l'empreinte approuvée d'un contrat, si le coffre la connaît.
pub async fn known(chain: &Chain, account: &str) -> Result<Option<String>> {
    let value = chain
        .view_call(accounts::VAULT, "known", &json!({"account_id": account}))
        .await
        .context("lecture de known() sur le coffre")?;
    Ok(value.as_str().map(str::to_string))
}

/// Extrait un montant stocké sous forme de chaîne ou de nombre.
pub fn u128_field(value: &Value, key: &str) -> u128 {
    u128_field_opt(value, key).unwrap_or(0)
}

/// Idem, mais rend `None` si le champ est absent.
pub fn u128_field_opt(value: &Value, key: &str) -> Option<u128> {
    let field = value.get(key)?;
    match field {
        Value::String(text) => text.parse().ok(),
        Value::Number(number) => number.as_u64().map(u128::from),
        other => amount_of(other),
    }
}
