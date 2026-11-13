//! Bibliothèque du crate dryrun-gate : garde-barrière, serveur HTTP, frise.

pub mod amount;
pub mod attack;
pub mod backend;
pub mod bench;
pub mod cardfmt;
pub mod cli;
pub mod demo;
pub mod gate;
pub mod http;
pub mod journal;
pub mod lang;
pub mod mcp;
#[cfg(feature = "real-sim")]
pub mod sim_real;
pub mod sim_stub;
pub mod sim_types;
pub mod summary;
pub mod testnet;

pub use amount::Amount;
pub use backend::{ExecOutcome, Simulator, VaultStatus};
pub use gate::{Gatekeeper, Outcome};
pub use http::{HttpServer, ProposeRequest};
pub use journal::{Journal, JournalEntry, JournalEntryView};
pub use sim_types::Caps;
