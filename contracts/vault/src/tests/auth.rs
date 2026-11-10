//! I4 et I7 : hiérarchie des autorités du coffre.
//!
//! - le propriétaire seul change les plafonds, les contrats connus et lève le gel ;
//! - la clé d'agent (compte du coffre) ne peut qu'exécuter ;
//! - la clé de gel (compte du coffre) ne peut que geler.

use crate::consts::{ERR_COOLDOWN, ERR_ONE_YOCTO, ERR_UNAUTHORIZED};
use crate::tests::setup::*;
use near_sdk::json_types::U128;

/// Empreinte de code factice.
const HASH: &str = "11111111111111111111111111111111";

#[test]
fn i4_stranger_cannot_touch_the_vault() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_stranger();

    assert_refused(
        attempt(|| fx.vault.set_caps(U128(1), U128(1))),
        ERR_UNAUTHORIZED,
    );
    assert_refused(
        attempt(|| fx.vault.approve_contract(acct(DRAINER), HASH.to_string())),
        ERR_UNAUTHORIZED,
    );
    assert_refused(
        attempt(|| fx.vault.forget_contract(acct(DRAINER))),
        ERR_UNAUTHORIZED,
    );
    assert_refused(attempt(|| fx.vault.unfreeze()), ERR_UNAUTHORIZED);
    assert_refused(attempt(|| fx.vault.freeze()), ERR_UNAUTHORIZED);
    assert_refused(
        attempt(|| fx.vault.set_sim_key("ed25519:abc".to_string())),
        ERR_UNAUTHORIZED,
    );
    // `execute` est réservé au compte du coffre : un étranger n'y touche pas.
    assert_refused(run(&mut fx, transfer(BOB, ONE_NEAR)), ERR_UNAUTHORIZED);
}

#[test]
fn i4_owner_needs_one_yocto_for_every_change() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_owner_without_yocto();

    assert_refused(
        attempt(|| fx.vault.set_caps(U128(1), U128(1))),
        ERR_ONE_YOCTO,
    );
    assert_refused(
        attempt(|| fx.vault.approve_contract(acct(DRAINER), HASH.to_string())),
        ERR_ONE_YOCTO,
    );
    assert_refused(attempt(|| fx.vault.unfreeze()), ERR_ONE_YOCTO);
}

#[test]
fn i7_guardian_key_can_only_freeze() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_agent();

    // Geler : oui.
    assert_accepted(attempt(|| fx.vault.freeze()));
    assert!(fx.vault.status().frozen);

    // Le reste : non.
    assert_refused(
        attempt(|| fx.vault.set_caps(U128(1), U128(1))),
        ERR_UNAUTHORIZED,
    );
    assert_refused(
        attempt(|| fx.vault.approve_contract(acct(DRAINER), HASH.to_string())),
        ERR_UNAUTHORIZED,
    );
    assert_refused(attempt(|| fx.vault.unfreeze()), ERR_UNAUTHORIZED);
    assert_refused(
        attempt(|| fx.vault.set_sim_key("ed25519:abc".to_string())),
        ERR_UNAUTHORIZED,
    );
}

#[test]
fn i4_owner_can_tighten_caps_but_loosening_waits() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_owner();

    // Resserrer passe tout de suite.
    assert_accepted(attempt(|| {
        fx.vault.set_caps(U128(ONE_NEAR), U128(5 * ONE_NEAR))
    }));
    assert_eq!(fx.vault.caps().tx_cap.0, ONE_NEAR);

    // Desserrer demande une heure.
    assert_refused(
        attempt(|| fx.vault.set_caps(U128(9 * ONE_NEAR), U128(5 * ONE_NEAR))),
        ERR_COOLDOWN,
    );
    fx.advance(60 * MINUTE_NS);
    assert_accepted(attempt(|| {
        fx.vault.set_caps(U128(9 * ONE_NEAR), U128(5 * ONE_NEAR))
    }));
    assert_eq!(fx.vault.caps().tx_cap.0, 9 * ONE_NEAR);
}

#[test]
fn i4_sim_key_is_stored_for_the_owner_only() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_owner()
        .vault
        .set_sim_key("ed25519:9Q8h2f".to_string());
    assert_eq!(
        fx.vault.status().sim_key,
        Some("ed25519:9Q8h2f".to_string())
    );
}
