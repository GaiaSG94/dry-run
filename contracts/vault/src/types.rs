//! Types du coffre : action de l'agent, vues JSON et objets d'état.

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::{near, AccountId, Gas};

/// Action proposée au coffre par la clé de l'agent.
///
/// Sérialisée en JSON avec les variantes externes (`{"Transfer": {...}}`).
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VaultAction {
    /// Sortie directe de NEAR vers un compte.
    Transfer {
        receiver_id: AccountId,
        amount: U128,
    },
    /// Appel de fonction sur un contrat, avec dépôt joint.
    Call {
        receiver_id: AccountId,
        method_name: String,
        args: String,
        deposit: U128,
        gas: Gas,
    },
}

impl VaultAction {
    /// Montant de NEAR que l'action fait sortir du coffre, en yoctoNEAR.
    pub fn amount(&self) -> u128 {
        match self {
            VaultAction::Transfer { amount, .. } => amount.0,
            VaultAction::Call { deposit, .. } => deposit.0,
        }
    }

    /// Compte visé par l'action.
    pub fn receiver(&self) -> &AccountId {
        match self {
            VaultAction::Transfer { receiver_id, .. } => receiver_id,
            VaultAction::Call { receiver_id, .. } => receiver_id,
        }
    }

    /// Étiquette lisible de l'action : `transfer` ou `call`.
    pub fn kind(&self) -> &'static str {
        match self {
            VaultAction::Transfer { .. } => "transfer",
            VaultAction::Call { .. } => "call",
        }
    }

    /// Vrai pour un appel de fonction : le seul cas qui exige un contrat connu.
    pub fn is_call(&self) -> bool {
        matches!(self, VaultAction::Call { .. })
    }

    /// Compte appelé si l'action est un appel de contrat.
    pub fn target_account(&self) -> Option<&AccountId> {
        match self {
            VaultAction::Call { receiver_id, .. } => Some(receiver_id),
            _ => None,
        }
    }

    /// Forme canonique de l'action, identique à celle calculée par le simulateur.
    ///
    /// C'est cette représentation qui est condensée dans le certificat : les
    /// deux côtés (garde-barrière et contrat) construisent le même objet.
    pub fn canonical_json(&self) -> serde_json::Value {
        match self {
            VaultAction::Transfer {
                receiver_id,
                amount,
            } => dryrun_core::types::canonical_action(
                "transfer",
                receiver_id.as_str(),
                "",
                "",
                amount.0,
                0,
            ),
            VaultAction::Call {
                receiver_id,
                method_name,
                args,
                deposit,
                gas,
            } => dryrun_core::types::canonical_action(
                "call",
                receiver_id.as_str(),
                method_name,
                args,
                deposit.0,
                gas.as_gas(),
            ),
        }
    }
}

/// Plafonds du coffre, en yoctoNEAR.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Caps {
    /// Plafond d'une sortie unique.
    pub tx_cap: U128,
    /// Plafond des sorties cumulées sur 24 heures glissantes.
    pub daily_cap: U128,
}

/// État complet du coffre, tel que le lisent le simulateur et la page web.
#[near(serializers = [json])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultStatus {
    /// Propriétaire du coffre.
    pub owner: AccountId,
    /// Vrai si les sorties sont gelées.
    pub frozen: bool,
    /// Date du gel, en nanosecondes (0 si le coffre n'est pas gelé).
    pub frozen_at: U128,
    /// Plafond par transaction.
    pub tx_cap: U128,
    /// Plafond sur 24 heures glissantes.
    pub daily_cap: U128,
    /// Dépensé sur les 24 dernières heures.
    pub spent_24h: U128,
    /// Nombre de contrats connus.
    pub known: u32,
    /// Nombre de clés d'agent observées.
    pub agent_keys: u32,
    /// Nombre de clés de gel observées.
    pub guardian_keys: u32,
    /// Clé de simulation déposée par le propriétaire (niveau V1).
    pub sim_key: Option<String>,
    /// Nombre d'entrées du journal.
    pub log_len: u32,
    /// Nombre de sorties comptées : l'identifiant de la prochaine est cette valeur.
    pub ops: U128,
    /// Nombre de sorties en attente de leur reçu.
    pub pending: u32,
}

/// Une entrée du journal du coffre.
#[near(serializers = [json, borsh])]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// Numéro d'ordre croissant (les entrées sortent par le plus petit).
    pub seq: U128,
    /// Date de l'entrée, en nanosecondes.
    pub at: U128,
    /// `executed`, `refused`, `frozen`, `unfrozen`, `caps`, `contract_approved`…
    pub kind: String,
    /// Compte visé, s'il y en a un.
    pub receiver: Option<AccountId>,
    /// Montant concerné, en yoctoNEAR.
    pub amount: U128,
    /// Code de refus, s'il y en a un.
    pub reason: Option<String>,
}

/// Contrat connu : empreinte de son code au moment de l'accord, et date.
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct KnownEntry {
    /// Empreinte du code approuvée par le propriétaire.
    pub code_hash: String,
    /// Date de l'accord, en nanosecondes.
    pub at: u64,
}

/// Dépense comptée dont on attend encore le reçu (annulable par `on_done`).
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct Pending {
    /// Case horaire où la dépense a été comptée.
    pub hour: u64,
    /// Montant compté, en yoctoNEAR.
    pub amount: u128,
    /// `transfer` ou `call`.
    pub kind: String,
    /// Compte visé.
    pub receiver: AccountId,
}
