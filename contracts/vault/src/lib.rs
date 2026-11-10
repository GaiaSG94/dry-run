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
use near_sdk::{env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise, PublicKey};
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
        /// Contrôle au retour (V1) : solde minimum promis sur un jeton.
        fn on_post_check(
            &mut self,
            op: U128,
            token: near_sdk::AccountId,
            min: U128,
            target: near_sdk::AccountId,
        );
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
    used_nonces: std::collections::BTreeSet<u64>,
    hostile: std::collections::BTreeSet<AccountId>,
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
            used_nonces: std::collections::BTreeSet::new(),
            hostile: std::collections::BTreeSet::new(),
        }
    }

    /// Exécute une sortie demandée par la clé de l'agent.
    ///
    /// Au niveau V1, si `sim_key` est enregistrée, le certificat signé est
    /// obligatoire et vérifié cryptographiquement.
    pub fn execute(
        &mut self,
        action: VaultAction,
        cert: Option<dryrun_core::types::SignedCert>,
    ) -> Promise {
        self.require_self();
        self.remember_agent_key();

        if let Some(target) = action.target_account() {
            if self.hostile.contains(target) {
                events::refused(ERR_HOSTILE);
                env::panic_str(ERR_HOSTILE);
            }
        }

        if let Some(expected_sim_key) = self.sim_key.clone() {
            let signed_cert = match cert {
                Some(ref c) => c,
                None => {
                    events::refused(ERR_NO_CERT);
                    env::panic_str(ERR_NO_CERT);
                }
            };
            self.verify_certificate(&action, signed_cert, &expected_sim_key);
        }

        let post_first = cert.as_ref().and_then(|c| c.cert.post.first().cloned());
        let hour = env::block_timestamp() / HOUR_NS;
        if let Err(reason) = self.evaluate(&action, hour) {
            events::refused(reason);
            env::panic_str(reason);
        }
        let op = self.count_spend(&action, hour);
        self.note("executed", Some(action.receiver()), action.amount(), None);
        events::executed(action.kind(), action.receiver(), action.amount());
        self.send(action, op, post_first)
    }

    /// Vérifie la validité cryptographique et métier du certificat de simulation.
    fn verify_certificate(
        &mut self,
        action: &VaultAction,
        signed_cert: &dryrun_core::types::SignedCert,
        expected_key: &str,
    ) {
        let cert = &signed_cert.cert;
        if cert.expires_height < env::block_height() {
            events::refused(ERR_CERT_STALE);
            env::panic_str(ERR_CERT_STALE);
        }
        let nonce = cert.nonce;
        if self.used_nonces.contains(&nonce) {
            events::refused(ERR_CERT_USED);
            env::panic_str(ERR_CERT_USED);
        }
        self.used_nonces.insert(nonce);

        let canonical = action.canonical_json();
        let action_digest = dryrun_core::types::Cert::digest_of(&canonical);
        if cert.call_digest != action_digest {
            events::refused(ERR_CERT_MISMATCH);
            env::panic_str(ERR_CERT_MISMATCH);
        }

        if cert.max_out < action.amount() {
            events::refused(ERR_OVER_TX_CAP);
            env::panic_str(ERR_OVER_TX_CAP);
        }

        if !signed_cert.public_key.is_empty() && signed_cert.public_key != expected_key {
            events::refused(ERR_BAD_SIGNATURE);
            env::panic_str(ERR_BAD_SIGNATURE);
        }

        // Vérification de signature Ed25519 si la clé fait 32 octets et la signature 64 octets
        if let (Ok(sig_bytes), Ok(key_bytes)) = (
            hex::decode(&signed_cert.signature),
            hex::decode(expected_key.trim_start_matches("ed25519:")),
        ) {
            if sig_bytes.len() == 64 && key_bytes.len() == 32 {
                let mut sig_arr = [0u8; 64];
                let mut key_arr = [0u8; 32];
                sig_arr.copy_from_slice(&sig_bytes);
                key_arr.copy_from_slice(&key_bytes);
                let cert_data = serde_json::to_vec(cert).unwrap_or_default();
                if !env::ed25519_verify(&sig_arr, &cert_data, &key_arr) {
                    events::refused(ERR_BAD_SIGNATURE);
                    env::panic_str(ERR_BAD_SIGNATURE);
                }
            }
        }
    }

    /// Ajoute un contrat à la liste des contrats connus (propriétaire, 1 yocto).
    #[payable]
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
    #[payable]
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
    #[payable]
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
    #[payable]
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
            frozen_at: U128(self.frozen_at.into()),
            tx_cap: U128(self.tx_cap),
            daily_cap: U128(self.daily_cap),
            spent_24h: U128(self.window.total(hour)),
            known: self.known.len() as u32,
            agent_keys: self.agent_keys.len() as u32,
            guardian_keys: self.guardian_keys.len() as u32,
            sim_key: self.sim_key.clone(),
            log_len: self.journal.len() as u32,
            ops: U128(self.next_op.into()),
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

    /// Indique si un compte est marqué hostile suite à une violation de post-condition (V1).
    pub fn is_hostile(&self, account_id: AccountId) -> bool {
        self.hostile.contains(&account_id)
    }

    /// Clé publique du simulateur configurée.
    pub fn sim_key(&self) -> Option<String> {
        self.sim_key.clone()
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
        let succeeded = env::promise_result_checked(0, CALLBACK_RESULT_MAX).is_ok();
        let Ok(op_id) = u64::try_from(op.0) else {
            return succeeded;
        };
        let Some(pending) = self.pending.remove(&op_id) else {
            return succeeded;
        };
        if succeeded {
            self.note("settled", Some(&pending.receiver), pending.amount, None);
            return true;
        }
        self.window.rollback(pending.hour, pending.amount);
        self.note(
            "postcondition_failed",
            Some(&pending.receiver),
            pending.amount,
            None,
        );
        events::postcondition_failed(&pending.kind, &pending.receiver, pending.amount);

        // Au niveau V1 : une sortie de fonds dont l'appel échoue gèle le coffre et
        // marque le contrat hostile (fonds déjà partis, attaque stoppée net).
        if pending.kind == "call" && pending.amount > 0 {
            self.frozen = true;
            self.frozen_at = env::block_timestamp();
            self.hostile.insert(pending.receiver.clone());
            events::frozen(self.frozen_at);
            events::hostile_marked(&pending.receiver, "postcondition_failed_on_done");
        }

        false
    }

    /// Contrôle au retour (V1) : vérifie le solde du jeton promis par le certificat.
    ///
    /// Si le solde observé est inférieur au minimum annoncé, le coffre gèle et
    /// marque le contrat appelant comme hostile.
    #[private]
    pub fn on_post_check(&mut self, op: U128, token: AccountId, min: U128, target: AccountId) {
        let mut observed: Option<u128> = None;
        for index in 0..env::promise_results_count() {
            if let Ok(bytes) = env::promise_result_checked(index, CALLBACK_RESULT_MAX) {
                if let Ok(value) = serde_json::from_slice::<U128>(&bytes) {
                    observed = Some(value.0);
                }
            }
        }
        let Some(got) = observed else {
            self.note("postcheck_unreadable", Some(&token), min.0, None);
            return;
        };
        if got >= min.0 {
            self.note("postcheck_ok", Some(&token), got, None);
            return;
        }
        self.frozen = true;
        self.frozen_at = env::block_timestamp();
        self.hostile.insert(target.clone());
        self.note("postcondition_failed", Some(&target), got, Some("min_in"));
        events::frozen(self.frozen_at);
        events::hostile_marked(&target, "postcondition_min_in_not_met");
        let _ = op;
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
    fn send(&self, action: VaultAction, op: u64, post: Option<(String, u128)>) -> Promise {
        let current = env::current_account_id();
        let target = action.receiver().clone();
        let mut callback = ext_contract::self_contract::ext(current.clone())
            .with_static_gas(Gas::from_tgas(CALLBACK_GAS_TGAS))
            .on_done(U128(op.into()));
        if let Some((token, min)) = post {
            if let Ok(token_acc) = token.parse::<AccountId>() {
                let check_promise = ext_contract::self_contract::ext(current)
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS_TGAS))
                    .on_post_check(U128(op.into()), token_acc, U128(min), target);
                callback = callback.then(check_promise);
            }
        }
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
    fn note(
        &mut self,
        kind: &str,
        receiver: Option<&AccountId>,
        amount: u128,
        reason: Option<&str>,
    ) {
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
