//! Contrôles rapides, sans bac à sable : fabrication des `Effects` et jugement.

use std::collections::BTreeMap;

use dryrun_core::types::KnownContracts;
use dryrun_core::{
    judge, AccountSnap, CodeEvent, Decision, Demande, Effects, FtDelta, Intent, KeyEvent,
    NearDelta, NftEvent, RuleCtx, Snapshot, Verdict,
};

use dryrun_sim::effects::{self, AccountSnapshot, ExecutionInfo, SnapshotSet};
use dryrun_sim::util::{near_amount, scope_of};
use dryrun_sim::vault_api::action_variants;

const NEAR: u128 = 10u128.pow(24);
const TEST: u128 = 10u128.pow(24);

/// Construit un état observé synthétique.
fn snapshot(accounts: &[(&str, u128, &str, u128)]) -> SnapshotSet {
    SnapshotSet {
        height: 10,
        block_hash: "11111111111111111111111111111111".to_string(),
        accounts: accounts
            .iter()
            .map(|(name, balance, code, ft)| AccountSnapshot {
                account: (*name).to_string(),
                balance: *balance,
                code_hash: (*code).to_string(),
                storage_usage: 1000,
                keys: Vec::new(),
                ft: if *ft == 0 {
                    Vec::new()
                } else {
                    vec![("test-token.test.near".to_string(), *ft)]
                },
            })
            .collect(),
    }
}

/// Un contexte de jugement conforme à la démo.
fn ctx(demand: Demande, intent: Intent) -> RuleCtx {
    RuleCtx {
        vault: "vault.test.near".to_string(),
        demand,
        intent,
        known: KnownContracts::new(),
        current_code: KnownContracts::new(),
        tx_cap: 2 * NEAR,
        daily_cap: 10 * NEAR,
        daily_spent: 0,
        fee_accounts: vec!["near".to_string(), "test.near".to_string()],
        ..RuleCtx::default()
    }
}

#[test]
fn un_transfert_honnete_donne_allow() {
    let vault_before = snapshot(&[
        ("vault.test.near", 50 * NEAR, "", 0),
        ("bob.test.near", 3 * NEAR, "", 0),
    ]);
    let vault_after = snapshot(&[
        ("vault.test.near", 49 * NEAR, "", 0),
        ("bob.test.near", 4 * NEAR, "", 0),
    ]);
    let info = ExecutionInfo::default();
    let produced = effects::effects(
        &vault_before,
        &vault_after,
        &info,
        "vault.test.near",
        &[],
        None,
    );
    assert_eq!(produced.near_delta_of("vault.test.near"), -(NEAR as i128));
    assert_eq!(produced.near_delta_of("bob.test.near"), NEAR as i128);

    let intent = Intent {
        summary: "payer 1 NEAR à bob".to_string(),
        beneficiaries: vec!["bob.test.near".to_string()],
        ..Intent::default()
    };
    let demand = Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: NEAR,
    };
    let decision = judge(&ctx(demand, intent), &produced);
    assert_eq!(
        decision.verdict,
        Verdict::Allow,
        "règles : {:?}",
        decision.rule_ids()
    );
}

#[test]
fn un_skimmer_est_bloque_par_r7_et_r8() {
    // Le contrat verse 20 % des jetons à mallory : le coffre reçoit moins que promis.
    let before = snapshot(&[
        ("vault.test.near", 50 * NEAR, "", 0),
        ("skimmer.test.near", 0, "code-skimmer", 0),
        ("mallory.test.near", 0, "", 0),
    ]);
    let after = snapshot(&[
        ("vault.test.near", 48 * NEAR, "", (TEST * 156) / 100),
        ("skimmer.test.near", 2 * NEAR, "code-skimmer", 0),
        ("mallory.test.near", 0, "", (TEST * 44) / 100),
    ]);
    let produced = effects::effects(
        &before,
        &after,
        &ExecutionInfo::default(),
        "vault.test.near",
        &[],
        None,
    );
    assert_eq!(
        produced.ft_delta_of("test-token.test.near", "mallory.test.near"),
        ((TEST * 44) / 100) as i128
    );

    let intent = Intent {
        summary: "2 NEAR contre au moins 1.9 TEST sur skimmer".to_string(),
        beneficiaries: vec!["vault.test.near".to_string()],
        min_in: vec![("test-token.test.near".to_string(), (TEST * 190) / 100)],
        ..Intent::default()
    };
    let demand = Demande::FunctionCall {
        contract: "skimmer.test.near".to_string(),
        method: "trap".to_string(),
        args: serde_json::json!({"note": "drain"}),
        deposit: 2 * NEAR,
        gas: 30_000_000_000_000,
    };
    let decision = judge(&ctx(demand, intent), &produced);
    assert_eq!(decision.verdict, Verdict::Block);
    let ids = decision.rule_ids();
    assert!(
        ids.contains(&"R7".to_string()),
        "R7 attendu, obtenu {ids:?}"
    );
    assert!(
        ids.contains(&"R8".to_string()),
        "R8 attendu, obtenu {ids:?}"
    );
}

#[test]
fn une_cle_ajoutee_est_bloquee_par_r2() {
    let mut before = snapshot(&[("vault.test.near", 50 * NEAR, "", 0)]);
    let mut after = before.clone();
    after.accounts[0].keys = vec![effects::KeyDesc {
        public_key: "ed25519:pirate".to_string(),
        scope: "full_access".to_string(),
    }];
    before.accounts[0].keys = vec![effects::KeyDesc {
        public_key: "ed25519:agent".to_string(),
        scope: "function_call:execute".to_string(),
    }];
    let produced = effects::effects(
        &before,
        &after,
        &ExecutionInfo::default(),
        "vault.test.near",
        &[],
        None,
    );
    assert!(matches!(
        produced.keys.first(),
        Some(KeyEvent::Added { .. })
    ));

    let demand = Demande::FunctionCall {
        contract: "drainer.test.near".to_string(),
        method: "trap".to_string(),
        args: serde_json::json!({}),
        deposit: NEAR,
        gas: 30_000_000_000_000,
    };
    let decision = judge(&ctx(demand, Intent::default()), &produced);
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.rule_ids().contains(&"R2".to_string()));
}

#[test]
fn le_simulateur_et_la_regle_pure_donnent_le_meme_verdict() {
    // Jeu d'effets synthétique, jugé deux fois : par la façade du simulateur
    // et par la règle pure de `dryrun-core`. Les verdicts doivent coïncider.
    for (montant, attendu) in [(NEAR, Verdict::Allow), (49 * NEAR, Verdict::Block)] {
        let before = snapshot(&[
            ("vault.test.near", 50 * NEAR, "", 0),
            ("bob.test.near", 0, "", 0),
        ]);
        let after = snapshot(&[
            ("vault.test.near", 50 * NEAR - montant, "", 0),
            ("bob.test.near", montant, "", 0),
        ]);
        let produced = effects::effects(
            &before,
            &after,
            &ExecutionInfo::default(),
            "vault.test.near",
            &[],
            None,
        );
        let intent = Intent {
            summary: format!("payer {} NEAR à bob", near_amount(montant)),
            beneficiaries: vec!["bob.test.near".to_string()],
            ..Intent::default()
        };
        let demand = Demande::Transfer {
            to: "bob.test.near".to_string(),
            amount: montant,
        };
        let pure: Decision = judge(&ctx(demand.clone(), intent.clone()), &produced);
        let facade: Decision = dryrun_sim::decide(&ctx(demand, intent), &produced);
        assert_eq!(pure.verdict, attendu);
        assert_eq!(pure, facade);
    }
}

#[test]
fn les_effets_decodent_les_evenements_nft() {
    let receipts = vec![
        dryrun_core::ReceiptInfo {
            id: "r1".to_string(),
            status: "success".to_string(),
            receiver: "test-nft.test.near".to_string(),
            logs: vec![
                "EVENT_JSON:{\"standard\":\"nep171\",\"version\":\"1.0.0\",\"event\":\"nft_approve\",\"data\":[{\"token_ids\":[\"7\"],\"owner_id\":\"vault.test.near\",\"approval_id\":\"x\"}]}".to_string(),
            ],
        },
        dryrun_core::ReceiptInfo {
            id: "r2".to_string(),
            status: "success".to_string(),
            receiver: "test-nft.test.near".to_string(),
            logs: vec!["nft_transfer token_id:7 to:mallory.test.near".to_string()],
        },
    ];
    let events = effects::nft_events(&receipts);
    assert!(matches!(events.first(), Some(NftEvent::Approve { token, .. }) if token == "7"));
    assert!(
        matches!(events.get(1), Some(NftEvent::Transfer { to, .. }) if to.starts_with("mallory"))
    );
}

#[test]
fn les_codes_et_les_stranded_sont_detectes() {
    let before = snapshot(&[
        ("vault.test.near", 50 * NEAR, "", 0),
        ("half-step.test.near", 0, "v1", 0),
    ]);
    let after = snapshot(&[
        ("vault.test.near", 48 * NEAR, "", 0),
        ("half-step.test.near", 2 * NEAR, "v2", 0),
    ]);
    let info = ExecutionInfo {
        receipts: vec![dryrun_core::ReceiptInfo {
            id: "r9".to_string(),
            status: "failed".to_string(),
            receiver: "half-step.test.near".to_string(),
            logs: vec![],
        }],
        gas_burnt: 5_000_000_000_000,
        height: 11,
        block_hash: "abc".to_string(),
    };
    let produced = effects::effects(&before, &after, &info, "vault.test.near", &[], None);
    assert!(
        produced.stranded,
        "un reçu échoue après une sortie de fonds"
    );
    assert!(matches!(
        produced.code.first(),
        Some(CodeEvent::Changed { .. })
    ));
    assert_eq!(produced.gas_burnt, 5_000_000_000_000);
    let decision = judge(
        &ctx(
            Demande::Transfer {
                to: "half-step.test.near".to_string(),
                amount: 2 * NEAR,
            },
            Intent::default(),
        ),
        &produced,
    );
    assert!(decision.rule_ids().contains(&"R9".to_string()));
}

#[test]
fn les_actions_du_coffre_sont_encodees_en_json() {
    let transfert = action_variants(&Demande::Transfer {
        to: "bob.test.near".to_string(),
        amount: NEAR,
    });
    assert_eq!(
        transfert[0],
        serde_json::json!({"Transfer": {"receiver_id": "bob.test.near", "amount": NEAR.to_string()}})
    );
    let appel = action_variants(&Demande::FunctionCall {
        contract: "good-dex.test.near".to_string(),
        method: "swap".to_string(),
        args: serde_json::json!({"min_out": "1"}),
        deposit: 2 * NEAR,
        gas: 30_000_000_000_000,
    });
    let call = appel[0].get("Call").expect("variante Call");
    assert_eq!(
        call.get("method_name").and_then(|v| v.as_str()),
        Some("swap")
    );
    assert!(call.get("gas").and_then(|v| v.as_str()).is_some());
}

#[test]
fn les_portees_de_cles_sont_lisibles() {
    use near_primitives::views::AccessKeyPermissionView;
    let full = AccessKeyPermissionView::FullAccess;
    let limited = AccessKeyPermissionView::FunctionCall {
        allowance: None,
        receiver_id: "vault.test.near".to_string(),
        method_names: vec!["execute".to_string()],
    };
    assert_eq!(scope_of(&full), "full_access");
    assert_eq!(scope_of(&limited), "function_call:execute");
}

#[test]
fn le_snapshot_et_les_deltas_sont_stables() {
    let snap = Snapshot {
        height: 1,
        block_hash: "b".to_string(),
        accounts: vec![AccountSnap {
            id: "a".to_string(),
            code_hash: "c".to_string(),
        }],
    };
    let produced = Effects {
        snapshot: snap,
        near_delta: vec![NearDelta {
            account: "a".to_string(),
            delta: -5,
        }],
        ft_delta: vec![FtDelta {
            token: "t".to_string(),
            account: "a".to_string(),
            delta: 2,
        }],
        ..Effects::default()
    };
    assert_eq!(produced.near_delta_of("a"), -5);
    assert_eq!(produced.ft_delta_of("t", "a"), 2);
    assert!(produced.has_outflow());
    let map: BTreeMap<String, String> = BTreeMap::new();
    assert!(map.is_empty());
}
