//! I1 : sur 24 heures glissantes, les sorties ne dépassent jamais le plafond du jour.

use crate::consts::{ERR_OVER_DAILY_CAP, ERR_OVER_TX_CAP};
use crate::tests::setup::*;
use crate::window::HOUR_NS;

#[test]
fn i1_daily_cap_holds_across_an_hour_boundary() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);

    // Heure 1 000 : deux sorties de 4 NEAR, à cheval sur la fin de l'heure.
    assert_accepted(run(&mut fx, transfer(BOB, 4 * ONE_NEAR)));
    fx.advance(59 * MINUTE_NS);
    assert_accepted(run(&mut fx, transfer(BOB, 4 * ONE_NEAR)));
    assert_eq!(fx.vault.spent_24h().0, 8 * ONE_NEAR);

    // Heure 1 001 : 3 NEAR de plus dépasseraient le plafond du jour.
    fx.advance(2 * MINUTE_NS);
    assert_refused(
        run(&mut fx, transfer(BOB, 3 * ONE_NEAR)),
        ERR_OVER_DAILY_CAP,
    );
    assert_eq!(fx.vault.spent_24h().0, 8 * ONE_NEAR);

    // 2 NEAR passent encore : on est pile au plafond.
    assert_accepted(run(&mut fx, transfer(BOB, 2 * ONE_NEAR)));
    assert_eq!(fx.vault.spent_24h().0, 10 * ONE_NEAR);

    // 23 h 30 plus tard, les 10 NEAR sont toujours dans la fenêtre.
    fx.jump(1_023 * HOUR_NS + 30 * MINUTE_NS);
    assert_eq!(fx.vault.spent_24h().0, 10 * ONE_NEAR);
    assert_refused(run(&mut fx, transfer(BOB, ONE_NEAR)), ERR_OVER_DAILY_CAP);

    // À 1 024 h 01, la case de 1 000 h est sortie : il ne reste que 2 NEAR.
    fx.jump(1_024 * HOUR_NS + MINUTE_NS);
    assert_eq!(fx.vault.spent_24h().0, 2 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 8 * ONE_NEAR)));
    assert_eq!(fx.vault.spent_24h().0, 10 * ONE_NEAR);
}

#[test]
fn i1_hourly_drip_never_overshoots() {
    let mut fx = Fixture::agent(ONE_NEAR, 10 * ONE_NEAR);
    let mut accepted = 0;

    // Une sortie de 1 NEAR par heure pendant 30 heures.
    for step in 0..30u64 {
        fx.jump(1_000 * HOUR_NS + step * HOUR_NS);
        if run(&mut fx, transfer(BOB, ONE_NEAR)).is_none() {
            accepted += 1;
        }
        assert!(
            fx.vault.spent_24h().0 <= 10 * ONE_NEAR,
            "la fenêtre a dépassé le plafond à l'heure {step}"
        );
    }

    // 10 heures d'affilée, puis un refus par heure tant que la fenêtre est
    // pleine ; à partir de l'heure 1 024 les premières cases se libèrent.
    assert_eq!(accepted, 16);
    assert_eq!(fx.vault.spent_24h().0, 10 * ONE_NEAR);
}

#[test]
fn i1_tx_cap_is_checked_before_the_daily_cap() {
    let mut fx = Fixture::agent(2 * ONE_NEAR, 10 * ONE_NEAR);
    assert_refused(run(&mut fx, transfer(BOB, 3 * ONE_NEAR)), ERR_OVER_TX_CAP);
    assert_accepted(run(&mut fx, transfer(BOB, 2 * ONE_NEAR)));
    assert_eq!(fx.vault.spent_24h().0, 2 * ONE_NEAR);
}

#[test]
fn i1_refused_action_leaves_the_window_untouched() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 4 * ONE_NEAR);
    assert_accepted(run(&mut fx, transfer(BOB, 3 * ONE_NEAR)));
    for _ in 0..3 {
        assert_refused(run(&mut fx, transfer(BOB, 2 * ONE_NEAR)), ERR_OVER_DAILY_CAP);
    }
    assert_eq!(fx.vault.spent_24h().0, 3 * ONE_NEAR);
    assert_eq!(fx.vault.log(0, 100).len(), 1);
}
