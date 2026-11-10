//! Contrat `vault` du pare-feu à simulation « Dry Run ».
//!
//! Le coffre détient les fonds et n'obéit qu'à trois autorités :
//! - le **propriétaire** (`owner`) règle les plafonds, approuve les contrats,
//!   dégèle et dépose la clé de simulation ;
//! - la **clé d'agent**, clé d'accès du compte du coffre limitée à `execute`,
//!   propose les sorties dans la limite des plafonds ;
//! - la **clé de gel**, clé d'accès limitée à `freeze`, ne peut que geler (I7).
//!
//! Le contrat n'ajoute **jamais** de clé d'accès au compte du coffre : les
//! seules clés qui existent sont celles que le propriétaire a posées hors
//! contrat, et toute mutation de clé est refusée par construction.
//!
//! Une sortie refusée émet `refused` puis provoque une panique : l'état reste
//! intact (la transaction est annulée) mais le code de refus reste lisible
//! dans les journaux du reçu.

pub mod consts;
pub mod events;
pub mod types;

mod journal;
mod window;

#[cfg(test)]
mod tests;

pub use consts::*;
pub use types::{Caps, LogEntry, VaultAction, VaultStatus};

use crate::journal::Journal;
use crate::types::{KnownEntry, Pending};
use crate::window::{Window, HOUR_NS};
use near_sdk::json_types::U128;
use near_sdk::{
    env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseResult, PublicKey,
};
use std::collections::BTreeMap;

/// Rappels asynchrones : le coffre s'appelle lui-même pour solder une sortie.
pub mod ext_contract {
    use near_sdk::ext_contract;
    use near_sdk::json_types::U128;

    /// Interface du rappel interne `on_done`.
    #[ext_contract(self_contract)]
    pub trait VaultSelf {
        /// Solder la sortie numéro `op` selon le résultat de la promesse.
        fn on_done(&mut self, op: U128);
    }
}

/// État du coffre.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Vault {
    owner: AccountId,
    tx_cap: u128,
    daily_cap: u128,
    /// Date du dernier changement de plafond (délai de desserrage).
    caps_at: u64,
    /// Fenêtre glissante des 24 heures (I1).
    window: Window,
    /// Contrats connus : compte → empreinte du code à l'accord.
    known: BTreeMap<AccountId, KnownEntry>,
    /// Clés d'agent observées sur `execute` (jamais ajoutées par le contrat).
    agent_keys: Vec<PublicKey>,
    /// Clés de gel observées sur `freeze`.
    guardian_keys: Vec<PublicKey>,
    frozen: bool,
    frozen_at: u64,
    /// Anneau des 100 dernières entrées.
    journal: Journal,
    /// Dépenses comptées en attente de leur reçu (annulables par `on_done`).
    pending: BTreeMap<u64, Pending>,
    next_op: u64,
    sim_key: Option<String>,
}

#[near]
impl Vault {
    /// Initialise le coffre.
    ///
    /// Le compte appelant doit être le propriétaire annoncé : personne d'autre
    /// ne peut s'approprier le contrat. Une seconde initialisation est refusée
    /// par l'attribut `#[init]` (l'état existe déjà).
    #[init]
    pub fn new(owner: AccountId, tx_cap: U128, daily_cap: U128) -> Self {
        if env::predecessor_account_id() != owner {
            env::panic_str(ERR_UNAUTHORIZED);
        }
        Self {
            owner,
            tx_cap: tx_cap.0,
            daily_cap: daily_cap.0,
            caps_at: env::block_timestamp(),
            window: Window::default(),
            known: BTreeMap::new(),
            agent_keys: Vec::new(),
            guardian_keys: Vec::new(),
            frozen: false,
            frozen_at: 0,
            journal: Journal::default(),
            pending: BTreeMap::new(),
            next_op: 0,
            sim_key: None,
        }
    }

    /// Exécute une sortie demandée par la clé de l'agent.
    ///
    /// La dépense est comptée **avant** l'envoi puis annulée par `on_done` si
    /// le reçu échoue (I5). Retourne la promesse sortante, suivie du rappel.
    pub fn execute(&mut self, action: VaultAction) -> Promise {
        self.require_self();
        self.remember_agent_key();
        let hour = env::block_timestamp() / HOUR_NS;
        if let Err(reason) = self.evaluate(&action, hour) {
            events::refused(reason);
            env::panic_str(reason);
        }
        let op = self.count_spend(&action, hour);
        self.note("executed", Some(action.receiver()), action.amount(), None);
        events::executed(action.kind(), action.receiver(), action.amount());
        self.send(action, op)
    }

    /// Ajoute un contrat à la liste des contrats connus (propriétaire, 1 yocto).
    pub fn approve_contract(&mut self, account_id: AccountId, code_hash: String) {
        self.require_owner();
        self.require_one_yocto();
        let at = env::block_timestamp();
        self.known.insert(
            account_id.clone(),
            KnownEntry {
                code_hash: code_hash.clone(),
                at,
            },
        );
        self.note("contract_approved", Some(&account_id), 0, None);
        events::contract_approved(&account_id, &code_hash);
    }

    /// Retire un contrat de la liste des contrats connus (propriétaire).
    pub fn forget_contract(&mut self, account_id: AccountId) {
        self.require_owner();
        self.known.remove(&account_id);
        self.note("contract_forgotten", Some(&account_id), 0, None);
    }

    /// Gèle le coffre : propriétaire **ou** clé de gel (I2, I7).
    pub fn freeze(&mut self) {
        self.require_owner_or_self();
        if env::predecessor_account_id() == env::current_account_id() {
            let key = env::signer_account_pk();
            remember_key(&mut self.guardian_keys, key);
        }
        if self.frozen {
            return;
        }
        let at = env::block_timestamp();
        self.frozen = true;
        self.frozen_at = at;
        self.note("frozen", None, 0, None);
        events::frozen(at);
    }

    /// Dégèle le coffre : propriétaire, 1 yoctoNEAR, après `UNFREEZE_DELAY_NS`.
    pub fn unfreeze(&mut self) {
        self.require_owner();
        self.require_one_yocto();
        let at = env::block_timestamp();
        if !self.frozen {
            return;
        }
        if at.saturating_sub(self.frozen_at) < UNFREEZE_DELAY_NS {
            events::refused(ERR_COOLDOWN);
            env::panic_str(ERR_COOLDOWN);
        }
        self.frozen = false;
        self.frozen_at = 0;
        self.note("unfrozen", None, 0, None);
        events::unfrozen(at);
    }

    /// Change les plafonds (propriétaire, 1 yoctoNEAR).
    ///
    /// Un resserrement s'applique tout de suite ; un desserrage attend
    /// `CAPS_DELAY_NS`, sinon `ERR_COOLDOWN`.
    pub fn set_caps(&mut self, tx_cap: U128, daily_cap: U128) {
        self.require_owner();
        self.require_one_yocto();
        let at = env::block_timestamp();
        let tighten = tx_cap.0 <= self.tx_cap && daily_cap.0 <= self.daily_cap;
        if !tighten && at.saturating_sub(self.caps_at) < CAPS_DELAY_NS {
            events::refused(ERR_COOLDOWN);
            env::panic_str(ERR_COOLDOWN);
        }
        self.tx_cap = tx_cap.0;
        self.daily_cap = daily_cap.0;
        self.caps_at = at;
        self.note("caps", None, 0, None);
    }

    /// Dépose la clé publique de simulation du niveau V1 (propriétaire, 1 yocto).
    pub fn set_sim_key(&mut self, public_key: String) {
        self.require_owner();
        self.require_one_yocto();
        self.sim_key = Some(public_key);
        self.note("sim_key", None, 0, None);
    }

    /// État complet du coffre.
    pub fn status(&self) -> VaultStatus {
        let hour = env::block_timestamp() / HOUR_NS;
        VaultStatus {
            owner: self.owner.clone(),
            frozen: self.frozen,
            frozen_at: U128(self.frozen_at),
            tx_cap: U128(self.tx_cap),
            daily_cap: U128(self.daily_cap),
            spent_24h: U128(self.window.total(hour)),
            known: self.known.len() as u32,
            agent_keys: self.agent_keys.len() as u32,
            guardian_keys: self.guardian_keys.len() as u32,
            sim_key: self.sim_key.clone(),
            log_len: self.journal.len() as u32,
            ops: U128(self.next_op),
            pending: self.pending.len() as u32,
        }
    }

    /// Les deux plafonds en cours.
    pub fn caps(&self) -> Caps {
        Caps {
            tx_cap: U128(self.tx_cap),
            daily_cap: U128(self.daily_cap),
        }
    }

    /// Dépensé sur les 24 heures glissantes.
    pub fn spent_24h(&self) -> U128 {
        U128(self.window.total(env::block_timestamp() / HOUR_NS))
    }

    /// Empreinte du code approuvée pour un compte, s'il est connu.
    pub fn known(&self, account_id: AccountId) -> Option<String> {
        self.known.get(&account_id).map(|e| e.code_hash.clone())
    }

    /// Contrôle les règles on-chain seules, sans rien exécuter.
    pub fn check(&self, action: VaultAction) -> bool {
        self.evaluate(&action, env::block_timestamp() / HOUR_NS)
            .is_ok()
    }

    /// Entrées du journal à partir du numéro `from`, au plus `limit`.
    pub fn log(&self, from: u64, limit: u32) -> Vec<LogEntry> {
        self.journal.entries_from(from, limit)
    }

    /// Rappel interne : annule la dépense comptée si le reçu a échoué (I5).
    #[private]
    pub fn on_done(&mut self, op: U128) -> bool {
        let succeeded = matches!(env::promise_result(0), PromiseResult::Successful(_));
        let Some(pending) = self.pending.remove(&op.0) else {
            return succeeded;
        };
        if succeeded {
            self.note("settled", Some(&pending.receiver), pending.amount, None);
        } else {
            self.window.rollback(pending.hour, pending.amount);
            self.note(
                "postcondition_failed",
                Some(&pending.receiver),
                pending.amount,
                None,
            );
            events::postcondition_failed(&pending.kind, &pending.receiver, pending.amount);
        }
        succeeded
    }
}

impl Vault {
    /// Le propriétaire seul.
    fn require_owner(&self) {
        if env::predecessor_account_id() != self.owner {
            env::panic_str(ERR_UNAUTHORIZED);
        }
    }

    /// Le coffre lui-même : derrière, la clé d'agent limitée à `execute`.
    fn require_self(&self) {
        if env::predecessor_account_id() != env::current_account_id() {
            env::panic_str(ERR_UNAUTHORIZED);
        }
    }

    /// Le propriétaire ou le coffre lui-même (clé de gel limitée à `freeze`).
    fn require_owner_or_self(&self) {
        let caller = env::predecessor_account_id();
        if caller != self.owner && caller != env::current_account_id() {
            env::panic_str(ERR_UNAUTHORIZED);
        }
    }

    /// Les actions du propriétaire exigent 1 yoctoNEAR de dépôt.
    fn require_one_yocto(&self) {
        if env::attached_deposit() != NearToken::from_yoctonear(ONE_YOCTO) {
            env::panic_str(ERR_ONE_YOCTO);
        }
    }

    /// Règles on-chain : gel, plafond par transaction, plafond 24 h, contrat connu.
    fn evaluate(&self, action: &VaultAction, hour: u64) -> Result<(), &'static str> {
        if self.frozen {
            return Err(ERR_FROZEN);
        }
        if action.amount() > self.tx_cap {
            return Err(ERR_OVER_TX_CAP);
        }
        if self.window.total(hour).saturating_add(action.amount()) > self.daily_cap {
            return Err(ERR_OVER_DAILY_CAP);
        }
        if action.is_call() && !self.known.contains_key(action.receiver()) {
            return Err(ERR_UNKNOWN_CONTRACT);
        }
        Ok(())
    }

    /// Compte la dépense et retient de quoi l'annuler si le reçu échoue.
    fn count_spend(&mut self, action: &VaultAction, hour: u64) -> u64 {
        self.window.add(hour, action.amount());
        let op = self.next_op;
        self.next_op = self.next_op.saturating_add(1);
        self.pending.insert(
            op,
            Pending {
                hour,
                amount: action.amount(),
                kind: action.kind().to_string(),
                receiver: action.receiver().clone(),
            },
        );
        op
    }

    /// Construit la promesse sortante, suivie du rappel `on_done`.
    fn send(&self, action: VaultAction, op: u64) -> Promise {
        let callback = ext_contract::self_contract::ext(env::current_account_id())
            .with_static_gas(Gas::from_tgas(CALLBACK_GAS_TGAS))
            .on_done(U128(op));
        match action {
            VaultAction::Transfer {
                receiver_id,
                amount,
            } => Promise::new(receiver_id)
                .transfer(NearToken::from_yoctonear(amount.0))
                .then(callback),
            VaultAction::Call {
                receiver_id,
                method_name,
                args,
                deposit,
                gas,
            } => Promise::new(receiver_id)
                .function_call(
                    method_name,
                    args.into_bytes(),
                    NearToken::from_yoctonear(deposit.0),
                    gas,
                )
                .then(callback),
        }
    }

    /// Retient la clé d'agent qui vient d'appeler `execute` (aucune clé n'est ajoutée).
    fn remember_agent_key(&mut self) {
        let key = env::signer_account_pk();
        remember_key(&mut self.agent_keys, key);
    }

    /// Ajoute une entrée au journal avec la date du bloc.
    fn note(&mut self, kind: &str, receiver: Option<&AccountId>, amount: u128, reason: Option<&str>) {
        let at = env::block_timestamp();
        self.journal.push(kind, receiver, amount, reason, at);
    }
}

/// Mémorise une clé observée, sans doublon et jusqu'à `MAX_TRACKED_KEYS`.
fn remember_key(keys: &mut Vec<PublicKey>, key: PublicKey) {
    if keys.len() < MAX_TRACKED_KEYS && !keys.contains(&key) {
        keys.push(key);
    }
}
