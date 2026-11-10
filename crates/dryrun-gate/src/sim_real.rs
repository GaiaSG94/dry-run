//! Adaptateur vers le vrai simulateur `dryrun-sim` (SPEC, section 4).
//!
//! Ce module n'est compilé qu'avec la feature `real-sim`. Il ne fait que
//! relancer l'API du crate voisin : aucune logique de simulation ici.

pub use dryrun_sim::{ExecOutcome, Simulator, VaultStatus};
