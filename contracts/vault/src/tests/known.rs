//! I3 : un appel vers un compte absent de `known` est refusé.

use crate::consts::ERR_UNKNOWN_CONTRACT;
use crate::tests::setup::*;

/// Empreinte de code factice du contrat piégé.
const DRAINER_HASH: &str = "11111111111111111111111111111111";

#[test]
fn i3_call_to_an_unknown_contract_is_refused() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert!(!fx.vault.check(call(DRAINER, "trap", ONE_NEAR)));
    assert_refused(
        run(&mut fx, call(DRAINER, "trap", ONE_NEAR)),
        ERR_UNKNOWN_CONTRACT,
    );
    assert_eq!(fx.vault.known(acct(DRAINER)), None);
    assert_eq!(fx.vault.spent_24h().0, 0);
}

#[test]
fn i3_approved_contract_is_accepted_then_forgotten() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    fx.as_owner()
        .vault
        .approve_contract(acct(DRAINER), DRAINER_HASH.to_string());
    assert_eq!(
        fx.vault.known(acct(DRAINER)),
        Some(DRAINER_HASH.to_string())
    );
    assert!(fx.vault.check(call(DRAINER, "trap", ONE_NEAR)));
    assert_eq!(fx.vault.status().known, 1);

    fx.as_agent();
    assert_accepted(run(&mut fx, call(DRAINER, "trap", ONE_NEAR)));

    // Oublié, le contrat redevient inconnu.
    fx.as_owner().vault.forget_contract(acct(DRAINER));
    assert_eq!(fx.vault.known(acct(DRAINER)), None);
    fx.as_agent();
    assert_refused(
        run(&mut fx, call(DRAINER, "trap", ONE_NEAR)),
        ERR_UNKNOWN_CONTRACT,
    );
}

#[test]
fn i3_plain_transfer_needs_no_known_contract() {
    let mut fx = Fixture::agent(10 * ONE_NEAR, 10 * ONE_NEAR);
    assert!(fx.vault.check(transfer(MALLORY, ONE_NEAR)));
    assert_accepted(run(&mut fx, transfer(MALLORY, ONE_NEAR)));
    assert_eq!(fx.vault.known(acct(MALLORY)), None);
}
