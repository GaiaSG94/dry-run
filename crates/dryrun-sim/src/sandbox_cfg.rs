//! Configuration commune des deux bacs à sable : blocs rapides, comptes de la démo.
//!
//! Un bloc toutes les 200 ms au lieu d'une seconde : le rejeu d'une demande
//! attend la fin des reçus (deux blocs), c'est le premier poste de latence.

use near_sandbox::{GenesisAccount, SandboxConfig};
use serde_json::json;

/// Bloc le plus rapide autorisé, en nanosecondes.
const MIN_BLOCK_NS: &str = "200000000";
/// Bloc le plus lent autorisé, en nanosecondes.
const MAX_BLOCK_NS: &str = "600000000";

/// Configuration d'un bac à sable : blocs rapides et comptes supplémentaires.
pub fn config(accounts: Vec<GenesisAccount>) -> SandboxConfig {
    SandboxConfig {
        additional_accounts: accounts,
        additional_genesis: Some(json!({
            "min_block_production_delay": MIN_BLOCK_NS,
            "max_block_production_delay": MAX_BLOCK_NS,
            "max_block_wait_delay": "3000000000",
        })),
        ..SandboxConfig::default()
    }
}
