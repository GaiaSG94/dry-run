//! Action du coffre, au format JSON attendu par `execute`.
//!
//! Le contrat `vault` prend une action (`Transfer` ou `Call`) et un certificat
//! optionnel. Sur le coffre testnet, aucune `sim_key` n'est enregistrée : le
//! certificat n'est pas exigé, il est donc transmis à `null`.

use dryrun_core::types::Demande;
use serde_json::{json, Value};

/// Rend l'argument JSON de `execute` : l'action et le certificat (nul).
pub fn execute_args(demand: &Demande) -> Value {
    let action = match demand {
        Demande::Transfer { to, amount } => json!({
            "Transfer": { "receiver_id": to, "amount": amount.to_string() }
        }),
        Demande::FunctionCall {
            contract,
            method,
            args,
            deposit,
            gas,
        } => json!({
            "Call": {
                "receiver_id": contract,
                "method_name": method,
                "args": args.to_string(),
                "deposit": deposit.to_string(),
                "gas": gas,
            }
        }),
    };
    json!({ "action": action, "cert": Value::Null })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfert_au_format_du_coffre() {
        let demand = Demande::Transfer {
            to: "testnet".to_string(),
            amount: 100_000_000_000_000_000_000_000,
        };
        let args = execute_args(&demand);
        assert_eq!(args["action"]["Transfer"]["receiver_id"], "testnet");
        assert_eq!(
            args["action"]["Transfer"]["amount"],
            "100000000000000000000000"
        );
        assert!(args["cert"].is_null());
    }

    #[test]
    fn appel_au_format_du_coffre() {
        let demand = Demande::FunctionCall {
            contract: "dex.test.near".to_string(),
            method: "swap".to_string(),
            args: json!({"token_out": "TEST"}),
            deposit: 2_000_000_000_000_000_000_000_000,
            gas: 30_000_000_000_000,
        };
        let args = execute_args(&demand);
        assert_eq!(args["action"]["Call"]["method_name"], "swap");
        assert_eq!(
            args["action"]["Call"]["deposit"],
            "2000000000000000000000000"
        );
        assert_eq!(args["action"]["Call"]["gas"], 30_000_000_000_000u64);
        assert!(args["cert"].is_null());
    }
}
