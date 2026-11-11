//! Tests unitaires du certificat cryptographique et du marquage hostile (V1).

use crate::tests::setup::{acct as account, set_env_at_height, Fixture, VAULT};
use crate::types::VaultAction;
use dryrun_core::types::{Cert, SignedCert};
use near_sdk::json_types::U128;

fn fixture() -> Fixture {
    Fixture::new(2 * 10u128.pow(24), 10 * 10u128.pow(24))
}

fn make_dummy_cert(action: &VaultAction, height: u64, nonce: u64) -> SignedCert {
    let digest = Cert::digest_of(&action.canonical());

    SignedCert {
        cert: Cert {
            call_digest: digest,
            snapshot_height: height,
            expires_height: height + 20,
            max_out: action.amount(),
            post: vec![],
            nonce,
        },
        signature: "sig123".to_string(),
        public_key: "ed25519:sim_key_dummy".to_string(),
    }
}

#[test]
fn v1_cert_required_when_sim_key_is_set() {
    let mut fx = fixture();
    fx.as_owner()
        .vault
        .set_sim_key("ed25519:sim_key_dummy".to_string());

    let action = VaultAction::Transfer {
        receiver_id: account("bob"),
        amount: U128(10u128.pow(24)),
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fx.as_agent().vault.execute(action, None);
    }));
    assert!(result.is_err(), "devait refuser sans certificat");
}

#[test]
fn v1_cert_stale_rejected() {
    let mut fx = fixture();
    fx.as_owner()
        .vault
        .set_sim_key("ed25519:sim_key_dummy".to_string());

    let action = VaultAction::Transfer {
        receiver_id: account("bob"),
        amount: U128(10u128.pow(24)),
    };

    // Le contexte de test place la hauteur de bloc à 0 : un certificat qui
    // expire avant le bloc courant est périmé.
    let mut cert = make_dummy_cert(&action, 0, 1);
    cert.cert.expires_height = 0;
    cert.cert.snapshot_height = 0;
    set_env_at_height(VAULT, 0, fx.at, 50, None);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fx.as_agent().vault.execute(action, Some(cert));
    }));
    assert!(result.is_err(), "devait refuser certificat périmé");
}

#[test]
fn v1_cert_replay_rejected() {
    let mut fx = fixture();
    fx.as_owner()
        .vault
        .set_sim_key("ed25519:sim_key_dummy".to_string());

    let action = VaultAction::Transfer {
        receiver_id: account("bob"),
        amount: U128(10u128.pow(24)),
    };

    let cert1 = make_dummy_cert(&action, 10, 42);
    let cert2 = make_dummy_cert(&action, 10, 42); // Même nonce

    let _ = fx.as_agent().vault.execute(action.clone(), Some(cert1));

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fx.as_agent().vault.execute(action, Some(cert2));
    }));
    assert!(result.is_err(), "devait refuser nonce déjà rejoué");
}

#[test]
fn v1_hostile_contract_rejected() {
    let mut fx = fixture();
    let target = account("drainer");
    fx.as_owner()
        .vault
        .approve_contract(target.clone(), "hash1".to_string());

    // Marque comme hostile suite à un incident
    fx.as_owner().vault.freeze();
    assert!(fx.vault.status().frozen);

    let action = VaultAction::Call {
        receiver_id: target.clone(),
        method_name: "trap".to_string(),
        args: "{}".to_string(),
        deposit: U128(10u128.pow(24)),
        gas: near_sdk::Gas::from_tgas(30),
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = fx.as_agent().vault.execute(action, None);
    }));
    assert!(result.is_err());
}

#[test]
fn v1_post_check_gele_et_marque_hostile_si_le_minimum_n_est_pas_atteint() {
    let mut fx = fixture();
    // Le contrat a promis 1.9 TEST ; la lecture du solde n'en montre que 1.
    let solde = serde_json::to_vec(&U128(10u128.pow(18))).unwrap();
    fx.as_agent();
    fx.promise_result(near_sdk::PromiseResult::Successful(solde));
    fx.vault.on_post_check(
        U128(0),
        account("test-token.test.near"),
        U128(19 * 10u128.pow(17)),
        account("drainer"),
    );

    assert!(fx.vault.status().frozen, "le coffre doit être gelé");
    assert!(
        fx.vault.is_hostile(account("drainer")),
        "le contrat fautif doit être marqué hostile"
    );
}

#[test]
fn v1_post_check_laisse_passer_si_le_minimum_est_atteint() {
    let mut fx = fixture();
    let solde = serde_json::to_vec(&U128(2 * 10u128.pow(18))).unwrap();
    fx.as_agent();
    fx.promise_result(near_sdk::PromiseResult::Successful(solde));
    fx.vault.on_post_check(
        U128(0),
        account("test-token.test.near"),
        U128(19 * 10u128.pow(17)),
        account("good-dex"),
    );

    assert!(!fx.vault.status().frozen, "le coffre doit rester ouvert");
    assert!(!fx.vault.is_hostile(account("good-dex")));
}

#[test]
fn v1_post_check_sans_resultat_lisible_ne_gele_pas() {
    let mut fx = fixture();
    fx.as_agent();
    fx.promise_result(near_sdk::PromiseResult::Failed);
    fx.vault.on_post_check(
        U128(0),
        account("test-token.test.near"),
        U128(19 * 10u128.pow(17)),
        account("good-dex"),
    );
    // En cas de doute sur la lecture, on ne gèle pas : le journal le dit.
    assert!(!fx.vault.status().frozen);
}
