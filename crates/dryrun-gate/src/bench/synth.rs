//! Synthèse des effets pour les cas de test du banc.
//!
//! Permet au banc de tester le moteur de règles sur les 200 cas en quelques
//! millisecondes sans avoir à démarrer 200 bacs à sable réels.

use super::corpus::{Case, Family};
use dryrun_core::rules::RuleCtx;
use dryrun_core::types::{
    AccountSnap, Effects, FtDelta, KeyEvent, NearDelta, NftEvent, ReceiptInfo, Snapshot,
};

/// Construit le contexte de règle et les effets synthétiques pour un cas.
pub fn scenario(case: &Case) -> (RuleCtx, Effects) {
    let mut ctx = super::base_ctx(case);
    let mut eff = Effects {
        snapshot: Snapshot {
            height: 100,
            block_hash: "hash123".to_string(),
            accounts: vec![AccountSnap {
                id: "vault.test.near".to_string(),
                code_hash: "vault-hash".to_string(),
            }],
        },
        ..Effects::default()
    };

    // Par défaut, good-dex et test-token sont connus
    ctx.known
        .insert("good-dex.test.near".to_string(), "dex-hash".to_string());
    ctx.known
        .insert("test-token.test.near".to_string(), "token-hash".to_string());

    match case.family {
        Family::HonestTransfer => {
            let to = match &case.demand {
                dryrun_core::Demande::Transfer { to, .. } => to.clone(),
                _ => "bob.test.near".to_string(),
            };
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
            eff.near_delta.push(NearDelta {
                account: to,
                delta: case.demand.amount() as i128,
            });
            eff.receipts.push(ReceiptInfo {
                id: "r1".to_string(),
                status: "success".to_string(),
                receiver: "bob.test.near".to_string(),
                logs: vec![],
            });
        }
        Family::HonestSwap => {
            let out = case.demand.amount();
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(out as i128),
            });
            eff.near_delta.push(NearDelta {
                account: "good-dex.test.near".to_string(),
                delta: out as i128,
            });
            let expected_token = case.intent.min_in.first().map(|(_, v)| *v).unwrap_or(out);
            eff.ft_delta.push(FtDelta {
                token: "TEST".to_string(),
                account: "vault.test.near".to_string(),
                delta: expected_token as i128,
            });
            eff.receipts.push(ReceiptInfo {
                id: "r1".to_string(),
                status: "success".to_string(),
                receiver: "good-dex.test.near".to_string(),
                logs: vec![],
            });
        }
        Family::HonestPayment => {
            let out = case.demand.amount();
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(out as i128),
            });
            eff.receipts.push(ReceiptInfo {
                id: "r1".to_string(),
                status: "success".to_string(),
                receiver: case.demand.target().unwrap_or("bob.test.near").to_string(),
                logs: vec![],
            });
        }
        Family::Injection | Family::DailyOverrun => {
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
        }
        Family::Drainer => {
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
            // Reçu 0 TEST -> R8 mord
        }
        Family::Skimmer => {
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
            // Tiers reçoit sans être déclaré
            eff.ft_delta.push(FtDelta {
                token: "TEST".to_string(),
                account: "mallory.test.near".to_string(),
                delta: 400_000_000_000_000_000_000_000,
            });
        }
        Family::HalfStep => {
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
            eff.stranded = true;
            eff.receipts.push(ReceiptInfo {
                id: "r_fail".to_string(),
                status: "failed".to_string(),
                receiver: "ghost.test.near".to_string(),
                logs: vec![],
            });
        }
        Family::Liar => {
            eff.near_delta.push(NearDelta {
                account: "vault.test.near".to_string(),
                delta: -(case.demand.amount() as i128),
            });
            // Prétend réussir mais solde FT inchangé -> R8
        }
        Family::NftApproval => {
            eff.nft.push(NftEvent::Approve {
                collection: "test-nft.test.near".to_string(),
                token: "1".to_string(),
                to: "mallory.test.near".to_string(),
            });
        }
        Family::UnknownContract => {
            // Contrat non présent dans ctx.known -> R5
        }
        Family::CodeChanged => {
            let target = case.demand.target().unwrap_or("shapeshifter.test.near");
            ctx.known
                .insert(target.to_string(), "original-hash".to_string());
            eff.snapshot.accounts.push(AccountSnap {
                id: target.to_string(),
                code_hash: "tampered-hash".to_string(),
            });
        }
        Family::KeyBackdoor => {
            eff.keys.push(KeyEvent::Added {
                account: "vault.test.near".to_string(),
                scope: "FullAccess".to_string(),
            });
        }
        Family::GasBomb => {
            eff.gas_burnt = 250_000_000_000_000; // > seuil R10
        }
    }

    (ctx, eff)
}
