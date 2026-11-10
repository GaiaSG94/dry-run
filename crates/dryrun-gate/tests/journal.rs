//! Tests unitaires du journal (frise chronologique).

use dryrun_core::types::{Decision, Demande, Effects, Intent, RuleHit, Verdict};
use dryrun_gate::journal::Journal;

/// Construit une décision factice pour les tests.
fn decision(verdict: Verdict) -> Decision {
    Decision {
        verdict,
        rules: vec![RuleHit {
            id: "R1".to_string(),
            title: "plafond".to_string(),
            detail: "détail".to_string(),
            severity: verdict,
        }],
        effects: Effects::default(),
    }
}

/// Construit une demande factice pour les tests.
fn transfer() -> Demande {
    Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: 10u128.pow(24),
    }
}

#[test]
fn journal_vide_au_depart() {
    let journal = Journal::new();
    assert!(journal.entries().is_empty());
}

#[test]
fn journal_conserve_l_ordre_d_ajout() {
    let journal = Journal::new();
    journal.record(
        transfer(),
        Intent::default(),
        &decision(Verdict::Allow),
        "carte-1".to_string(),
        true,
    );
    journal.record(
        transfer(),
        Intent::default(),
        &decision(Verdict::Block),
        "carte-2".to_string(),
        false,
    );

    let entries = journal.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].id, 1);
    assert_eq!(entries[1].id, 2);
    assert_eq!(entries[0].verdict, Verdict::Allow);
    assert_eq!(entries[1].verdict, Verdict::Block);
    assert_eq!(entries[1].card, "carte-2");
    assert!(!entries[1].executed);
}

#[test]
fn journal_clear_vide_tout() {
    let journal = Journal::new();
    journal.record(
        transfer(),
        Intent::default(),
        &decision(Verdict::Warn),
        "c".to_string(),
        false,
    );
    journal.clear();
    assert!(journal.entries().is_empty());

    // Après clear, les identifiants repartent de 1.
    let entry = journal.record(
        transfer(),
        Intent::default(),
        &decision(Verdict::Allow),
        "d".to_string(),
        true,
    );
    assert_eq!(entry.id, 1);
}
