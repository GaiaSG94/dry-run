//! Événements NEP-297 : chaque décision du coffre laisse une trace lisible.

use crate::consts::{ERR_FROZEN, ERR_OVER_DAILY_CAP};
use crate::tests::setup::*;
use near_sdk::test_utils::get_logs;

/// Vrai si les journaux du reçu contiennent l'événement demandé.
fn emitted(event: &str) -> bool {
    let needle = format!("\"event\":\"{event}\"");
    get_logs().iter().any(|line| line.contains(&needle))
}

#[test]
fn events_follow_nep297() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, ONE_NEAR)));
    let journal = get_logs().join("\n");
    assert!(journal.contains("\"standard\":\"nep297\""), "{journal}");
    assert!(journal.contains("\"version\":\"1.0.0\""), "{journal}");
    assert!(emitted("executed"), "{journal}");
    assert!(
        journal.contains("\"receiver\":\"bob.test.near\""),
        "{journal}"
    );
    assert!(journal.contains("\"kind\":\"transfer\""), "{journal}");
}

#[test]
fn events_report_a_refusal_with_its_code() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 4 * ONE_NEAR)));
    assert_refused(
        run(&mut fx, transfer(BOB, 7 * ONE_NEAR)),
        ERR_OVER_DAILY_CAP,
    );
    let journal = get_logs().join("\n");
    assert!(emitted("refused"), "{journal}");
    assert!(journal.contains(ERR_OVER_DAILY_CAP), "{journal}");
}

#[test]
fn events_cover_freeze_unfreeze_and_approval() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.vault.freeze();
    assert!(emitted("frozen"));

    fx.as_owner().advance(60 * MINUTE_NS);
    assert_accepted(attempt(|| fx.vault.unfreeze()));
    assert!(emitted("unfrozen"));

    fx.vault.approve_contract(acct(DRAINER), "abc".to_string());
    assert!(emitted("contract_approved"));
    assert!(get_logs().join("\n").contains("abc"));
}

#[test]
fn events_report_a_failed_postcondition() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, ONE_NEAR)));
    fx.promise_result(near_sdk::PromiseResult::Failed);
    assert!(!fx.vault.on_done(near_sdk::json_types::U128(0)));
    assert!(emitted("postcondition_failed"));
}

#[test]
fn a_frozen_vault_still_logs_its_refusal() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.vault.freeze();
    assert_refused(run(&mut fx, transfer(BOB, ONE_NEAR)), ERR_FROZEN);
    assert!(emitted("refused"));
}
