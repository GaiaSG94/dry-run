//! I5 : la dépense comptée égale la somme des sorties réellement réussies.

use crate::tests::setup::*;
use near_sdk::json_types::U128;
use near_sdk::PromiseResult;

#[test]
fn i5_failed_promise_rolls_back_the_counted_spend() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 5 * ONE_NEAR)));

    // La sortie est comptée tout de suite, en attendant le reçu.
    assert_eq!(fx.vault.spent_24h().0, 5 * ONE_NEAR);
    let status = fx.vault.status();
    assert_eq!(status.pending, 1);
    assert_eq!(status.ops.0, 1);

    // Le reçu échoue : `on_done` annule la dépense.
    fx.promise_result(PromiseResult::Failed);
    assert!(!fx.vault.on_done(U128(0)));
    assert_eq!(fx.vault.spent_24h().0, 0);
    assert_eq!(fx.vault.status().pending, 0);
}

#[test]
fn i5_successful_promise_keeps_the_spend() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 5 * ONE_NEAR)));
    fx.promise_result(PromiseResult::Successful(Vec::new()));
    assert!(fx.vault.on_done(U128(0)));
    assert_eq!(fx.vault.spent_24h().0, 5 * ONE_NEAR);
    assert_eq!(fx.vault.status().pending, 0);
}

#[test]
fn i5_spend_equals_the_sum_of_successful_outflows() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    let mut reussies = 0u128;

    // Trois sorties de 2 NEAR : les deux premières réussissent, la dernière échoue.
    let results = [
        PromiseResult::Successful(Vec::new()),
        PromiseResult::Successful(Vec::new()),
        PromiseResult::Failed,
    ];
    for (op, result) in results.into_iter().enumerate() {
        assert_accepted(run(&mut fx, transfer(BOB, 2 * ONE_NEAR)));
        fx.promise_result(result);
        if fx.vault.on_done(U128(op as u128)) {
            reussies += 2 * ONE_NEAR;
        }
        assert_eq!(fx.vault.spent_24h().0, reussies);
    }
    assert_eq!(fx.vault.spent_24h().0, 4 * ONE_NEAR);
}

#[test]
fn i5_rollback_frees_room_under_the_daily_cap() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 4 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 3 * ONE_NEAR)));
    fx.promise_result(PromiseResult::Failed);
    assert!(!fx.vault.on_done(U128(0)));
    assert_eq!(fx.vault.spent_24h().0, 0);

    // La place libérée est réutilisable dans la même fenêtre.
    assert_accepted(run(&mut fx, transfer(BOB, 4 * ONE_NEAR)));
    assert_eq!(fx.vault.spent_24h().0, 4 * ONE_NEAR);
}

#[test]
fn i5_unknown_operation_is_harmless() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.promise_result(PromiseResult::Successful(Vec::new()));
    assert!(fx.vault.on_done(U128(42)));
    assert_eq!(fx.vault.spent_24h().0, 0);
}
