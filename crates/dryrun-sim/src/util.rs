//! Petits utilitaires partagés (montants, portée des clés).

use serde_json::Value;

use near_primitives::views::AccessKeyPermissionView;

/// Extrait un montant d'une réponse `{"balance": "123"}`, `{"amount": "..."}` ou `"123"`.
pub fn amount_of(value: &Value) -> Option<u128> {
    let raw = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        other => other
            .get("balance")
            .or_else(|| other.get("amount"))
            .and_then(Value::as_str)?
            .to_string(),
    };
    raw.parse().ok()
}

/// Portée lisible d'une clé d'accès : `full_access` ou `function_call:execute`.
pub fn scope_of(permission: &AccessKeyPermissionView) -> String {
    match permission {
        AccessKeyPermissionView::FullAccess => "full_access".to_string(),
        AccessKeyPermissionView::FunctionCall { method_names, .. } => {
            format!("function_call:{}", method_names.join("+"))
        }
        AccessKeyPermissionView::GasKeyFullAccess { .. } => "gas_key:full_access".to_string(),
        AccessKeyPermissionView::GasKeyFunctionCall { method_names, .. } => {
            format!("gas_key:{}", method_names.join("+"))
        }
    }
}

/// Formate un montant en yoctoNEAR de façon lisible (6 décimales de NEAR).
pub fn near_amount(yocto: u128) -> String {
    let whole = yocto / 10u128.pow(24);
    let frac = (yocto % 10u128.pow(24)) / 10u128.pow(18);
    format!("{whole}.{frac:06}")
}
