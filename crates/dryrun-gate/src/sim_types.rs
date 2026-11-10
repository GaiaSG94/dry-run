//! Types d'état du coffre et de résultat d'exécution partagés par le crate.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::amount::Amount;

/// Plafonds du coffre.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Caps {
    /// Plafond par transaction.
    pub tx_cap: Amount,
    /// Plafond sur 24 heures glissantes.
    pub daily_cap: Amount,
}

/// État visible du coffre, tel que lu sur la chaîne.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VaultStatus {
    /// Compte du coffre.
    pub account_id: String,
    /// Solde NEAR disponible.
    pub balance: Amount,
    /// Plafond par transaction.
    pub tx_cap: Amount,
    /// Plafond sur 24 heures glissantes.
    pub daily_cap: Amount,
    /// Dépensé sur la fenêtre glissante.
    pub spent_24h: Amount,
    /// Vrai si le coffre est gelé.
    pub is_frozen: bool,
    /// Propriétaire du coffre (seul à pouvoir changer plafonds et contrats).
    pub owner: String,
    /// Contrats connus : compte → empreinte du code à l'accord.
    pub known_contracts: BTreeMap<String, String>,
    /// Simulateur actif : `dryrun-sim` ou `stub`.
    #[serde(default)]
    pub backend: String,
}

/// Résultat d'une exécution réelle sur la chaîne.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecOutcome {
    /// Vrai si la transaction a été appliquée.
    pub success: bool,
    /// Empreinte de la transaction.
    pub tx_hash: String,
    /// Gas brûlé par la transaction.
    pub gas_burnt: u64,
    /// Journaux émis par les contrats.
    pub logs: Vec<String>,
}

/// Contrats déployés lors du bootstrap de la démonstration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Deployed {
    /// Compte → empreinte du code déployé.
    pub contracts: BTreeMap<String, String>,
}
