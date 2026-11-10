//! Contrat JSON : les formes exactes attendues par le simulateur et le garde-barrière.

use crate::tests::setup::*;
use crate::VaultAction;
use serde_json::Value;

#[test]
fn action_json_is_externally_tagged() {
    let action = transfer(BOB, ONE_NEAR);
    let text = serde_json::to_string(&action).expect("sérialisation de l'action");
    assert_eq!(
        text,
        r#"{"Transfer":{"receiver_id":"bob.test.near","amount":"1000000000000000000000000"}}"#
    );
    let back: VaultAction = serde_json::from_str(&text).expect("relecture de l'action");
    assert_eq!(back, action);
}

#[test]
fn call_json_carries_method_args_deposit_and_gas_as_strings() {
    let action = call(DRAINER, "trap", ONE_NEAR);
    let text = serde_json::to_string(&action).expect("sérialisation de l'appel");
    let parsed: Value = serde_json::from_str(&text).expect("JSON valide");
    let call = &parsed["Call"];
    assert_eq!(call["receiver_id"], "drainer.test.near");
    assert_eq!(call["method_name"], "trap");
    assert_eq!(call["args"], "{}");
    assert_eq!(call["deposit"], ONE_NEAR.to_string());
    assert_eq!(call["gas"], "30000000000000");
}

#[test]
fn status_json_exposes_the_whole_vault() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert!(run(&mut fx, transfer(BOB, ONE_NEAR)).is_none());
    let status = serde_json::to_value(fx.vault.status()).expect("statut en JSON");

    assert_eq!(status["owner"], OWNER);
    assert_eq!(status["frozen"], false);
    assert_eq!(status["tx_cap"], (10 * ONE_NEAR).to_string());
    assert_eq!(status["daily_cap"], (10 * ONE_NEAR).to_string());
    assert_eq!(status["spent_24h"], ONE_NEAR.to_string());
    assert_eq!(status["known"], 0);
    assert_eq!(status["ops"], "1");
    assert_eq!(status["pending"], 1);
    assert_eq!(status["log_len"], 1);
    assert_eq!(status["sim_key"], Value::Null);
}

#[test]
fn caps_endpoint_returns_both_caps() {
    let fx = Fixture::agent(2 * ONE_NEAR, 20 * ONE_NEAR);
    let text = serde_json::to_string(&fx.vault.caps()).expect("plafonds en JSON");
    assert_eq!(
        text,
        format!(
            r#"{{"tx_cap":"{}","daily_cap":"{}"}}"#,
            2 * ONE_NEAR,
            20 * ONE_NEAR
        )
    );
}

#[test]
fn log_entries_are_readable_by_the_web_page() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert!(run(&mut fx, transfer(BOB, 3 * ONE_NEAR)).is_none());
    let entries = fx.vault.log(0, 10);
    assert_eq!(entries.len(), 1);

    let entry = serde_json::to_value(&entries[0]).expect("entrée en JSON");
    assert_eq!(entry["seq"], "0");
    assert_eq!(entry["kind"], "executed");
    assert_eq!(entry["receiver"], BOB);
    assert_eq!(entry["amount"], (3 * ONE_NEAR).to_string());
    assert_eq!(entry["reason"], Value::Null);
    assert_eq!(entry["at"], u128::from(fx.at).to_string());

    // Au-delà de la première entrée, plus rien.
    assert!(fx.vault.log(1, 10).is_empty());
}
