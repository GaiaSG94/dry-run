//! Types partagés du pare-feu à simulation « Dry Run ».
//!
//! Ce crate ne dépend que de `serde` : il est partagé par le simulateur,
//! le garde-barrière, la page web et les tests.

pub mod card;
pub mod rules;
pub mod types;

pub use rules::{judge, RuleCtx, RULES};
pub use types::{
    AccountSnap, CodeEvent, Decision, Demande, Effects, FtDelta, Intent, KeyEvent, NearDelta,
    NftEvent, ReceiptInfo, RuleHit, Snapshot, Verdict,
};
