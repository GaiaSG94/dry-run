//! I2 : aucune sortie tant que le coffre est gelé, et la clé de gel fonctionne.

use crate::consts::{ERR_COOLDOWN, ERR_FROZEN};
use crate::tests::setup::*;
use crate::window::HOUR_NS;

#[test]
fn i2_frozen_vault_refuses_every_outflow() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.vault.freeze();
    assert!(fx.vault.status().frozen);
    assert!(!fx.vault.check(transfer(BOB, ONE_NEAR)));
    assert!(!fx.vault.check(call(DRAINER, "trap", ONE_NEAR)));

    assert_refused(run(&mut fx, transfer(BOB, ONE_NEAR)), ERR_FROZEN);
    assert_eq!(fx.vault.spent_24h().0, 0);
    assert_eq!(fx.vault.status().pending, 0);
}

#[test]
fn i2_guardian_key_can_freeze() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    // La clé de gel est une clé d'accès du compte du coffre : l'appelant est
    // donc le coffre lui-même.
    fx.as_agent().vault.freeze();
    assert!(fx.vault.status().frozen);
    assert_eq!(fx.vault.status().guardian_keys, 1);
    assert_eq!(fx.vault.status().frozen_at.0, u128::from(fx.at));
    assert_refused(run(&mut fx, transfer(BOB, ONE_NEAR)), ERR_FROZEN);
}

#[test]
fn i2_owner_unfreezes_after_the_delay_only() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.vault.freeze();

    // Trop tôt : le dégel attend UNFREEZE_DELAY_NS.
    fx.as_owner().advance(30 * MINUTE_NS);
    assert_refused(attempt(|| fx.vault.unfreeze()), ERR_COOLDOWN);
    assert!(fx.vault.status().frozen);

    // Une heure après le gel, le dégel passe.
    fx.advance(30 * MINUTE_NS);
    assert_accepted(attempt(|| fx.vault.unfreeze()));
    assert!(!fx.vault.status().frozen);
    assert_eq!(fx.vault.status().frozen_at.0, 0);

    fx.as_agent();
    assert_accepted(run(&mut fx, transfer(BOB, ONE_NEAR)));
}

#[test]
fn i2_freeze_is_idempotent() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.vault.freeze();
    let frozen_at = fx.vault.status().frozen_at.0;
    fx.advance(HOUR_NS);
    fx.vault.freeze();
    assert_eq!(fx.vault.status().frozen_at.0, frozen_at);
    assert_eq!(fx.vault.log(0, 100).len(), 1);
}
