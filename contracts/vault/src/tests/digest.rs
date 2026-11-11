//! Vérifie que le contrat et le simulateur condensent **exactement** la même
//! demande : c'est la condition pour qu'un certificat soit accepté.

use crate::types::VaultAction;
use dryrun_core::types::{Cert, Demande};
use near_sdk::json_types::U128;

#[test]
fn le_digest_du_contrat_egal_celui_du_simulateur() {
    let action = VaultAction::Transfer {
        receiver_id: "bob.test.near".parse().unwrap(),
        amount: U128(10u128.pow(24)),
    };
    let demand = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 10u128.pow(24),
    };

    assert_eq!(
        action.canonical(),
        demand.canonical(),
        "les deux formes canoniques doivent être identiques"
    );
    assert_eq!(
        Cert::digest_of(&action.canonical()),
        Cert::compute_digest(&demand)
    );
}

#[test]
fn le_digest_du_contrat_egal_celui_du_simulateur_pour_un_appel() {
    let action = VaultAction::Call {
        receiver_id: "good-dex.test.near".parse().unwrap(),
        method_name: "swap".to_string(),
        args: "{\"token_out\":\"TEST\",\"min_out\":\"1900000000000000000\"}".to_string(),
        deposit: U128(2 * 10u128.pow(24)),
        gas: near_sdk::Gas::from_tgas(30),
    };
    let demand = Demande::FunctionCall {
        contract: "good-dex.test.near".to_string(),
        method: "swap".to_string(),
        args: serde_json::json!({"token_out": "TEST", "min_out": "1900000000000000000"}),
        deposit: 2 * 10u128.pow(24),
        gas: 30_000_000_000_000,
    };

    assert_eq!(action.canonical(), demand.canonical());
    assert_eq!(
        Cert::digest_of(&action.canonical()),
        Cert::compute_digest(&demand)
    );
}
