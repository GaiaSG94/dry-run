//! Tests de la simulation différentielle et des certificats de simulation (V1).

use dryrun_core::types::{Cert, Demande, Effects, FtDelta, Intent, NearDelta, Verdict};
use dryrun_sim::cert::CertSigner;
use dryrun_sim::differential::{self, scaled};

#[test]
fn signer_generates_verifiable_certificate() {
    let signer = CertSigner::new();
    let demand = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 10u128.pow(24),
    };
    let cert = Cert {
        call_digest: Cert::compute_digest(&demand),
        snapshot_height: 100,
        expires_height: 120,
        max_out: 10u128.pow(24),
        post: vec![],
        nonce: 1,
    };

    let signed = signer.sign(cert);
    assert_eq!(signed.public_key, signer.public_key_hex());
    assert_eq!(signed.signature.len(), 128); // 64 octets en hex
}

#[test]
fn differential_scaling_divides_amount_correctly() {
    let demand = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 2 * 10u128.pow(24),
    };
    let small = scaled(&demand, 100);
    assert_eq!(small.amount(), 2 * 10u128.pow(22));
}

#[test]
fn differential_detects_divergent_behavior() {
    let vault = "vault.test.near";
    let intent = Intent {
        summary: "échanger 2 NEAR contre TEST".to_string(),
        beneficiaries: vec!["dex.test.near".to_string()],
        min_in: vec![("TEST".to_string(), 10u128.pow(24))],
        max_out: None,
    };

    // Au montant normal : 2 NEAR sortent, 0 TEST reçu (piège !)
    let mut full = Effects::default();
    full.near_delta.push(NearDelta {
        account: vault.to_string(),
        delta: -(2 * 10i128.pow(24)),
    });

    // Au petit montant : 0.02 NEAR sortent, 0.02 TEST reçu (il se comporte bien pour tromper le simulateur !)
    let mut small = Effects::default();
    small.near_delta.push(NearDelta {
        account: vault.to_string(),
        delta: -(2 * 10i128.pow(22)),
    });
    small.ft_delta.push(FtDelta {
        token: "TEST".to_string(),
        account: vault.to_string(),
        delta: 2 * 10i128.pow(22),
    });

    let report = differential::compare(vault, &intent, &full, &small);
    assert!(report.asymmetric);
    let hit = differential::differential_hit(&report).expect("devait lever R12");
    assert_eq!(hit.id, "R12");
    assert_eq!(hit.severity, Verdict::Block);
}
