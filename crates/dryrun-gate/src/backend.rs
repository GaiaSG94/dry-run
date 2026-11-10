//! Sélection de l'implémentation du simulateur.
//!
//! - feature `real-sim` : branche le vrai `dryrun_sim::Simulator` (section 4 du SPEC) ;
//! - feature `stub` (défaut) : branche `crate::sim_stub::Simulator`, un simulateur
//!   en mémoire qui respecte exactement la même API, pour compiler et tester seul.

#[cfg(feature = "real-sim")]
mod selected {
    pub use crate::sim_real::{ExecOutcome, Simulator, VaultStatus};
}

#[cfg(not(feature = "real-sim"))]
mod selected {
    pub use crate::sim_stub::Simulator;
    pub use crate::sim_types::{ExecOutcome, VaultStatus};
}

pub use selected::{ExecOutcome, Simulator, VaultStatus};

/// Vrai si le crate compile avec le vrai simulateur `dryrun-sim`.
pub const IS_REAL_SIM: bool = cfg!(feature = "real-sim");