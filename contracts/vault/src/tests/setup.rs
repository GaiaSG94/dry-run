//! Outils partagés par les tests unitaires du coffre.
//!
//! Les tests tournent sur le faux bac à sable de near-sdk : aucun réseau, aucun
//! fichier wasm, juste un contexte de bloc que l'on fait avancer à la main.

use crate::window::HOUR_NS;
use crate::{Vault, VaultAction};
use near_sdk::json_types::U128;
use near_sdk::test_utils::VMContextBuilder;
use near_sdk::{testing_env, AccountId, Gas, NearToken, PromiseResult, RuntimeFeesConfig};
use std::any::Any;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Propriétaire du coffre.
pub const OWNER: &str = "owner.test.near";
/// Compte du coffre : il porte la clé d'agent et la clé de gel.
pub const VAULT: &str = "vault.test.near";
/// Bénéficiaire honnête.
pub const BOB: &str = "bob.test.near";
/// Contrat piégé de la démonstration.
pub const DRAINER: &str = "drainer.test.near";
/// Compte étranger quelconque.
pub const MALLORY: &str = "mallory.test.near";
/// Une minute en nanosecondes.
pub const MINUTE_NS: u64 = 60_000_000_000;
/// Un NEAR en yoctoNEAR.
pub const ONE_NEAR: u128 = 10u128.pow(24);

/// Identifiant de compte à partir d'une chaîne.
pub fn acct(id: &str) -> AccountId {
    id.parse().expect("identifiant de compte valide")
}

/// Coffre de test et contexte d'appel courant.
pub struct Fixture {
    pub vault: Vault,
    /// Date du bloc simulé, en nanosecondes.
    pub at: u64,
    caller: String,
    deposit: u128,
}

impl Fixture {
    /// Coffre neuf, déployé par le propriétaire, avec les deux plafonds donnés.
    pub fn new(tx_cap: u128, daily_cap: u128) -> Self {
        let at = 1_000 * HOUR_NS;
        set_env(OWNER, 0, at, None);
        let vault = Vault::new(acct(OWNER), U128(tx_cap), U128(daily_cap));
        Fixture {
            vault,
            at,
            caller: OWNER.to_string(),
            deposit: 0,
        }
    }

    /// Coffre prêt pour un test d'agent : le compte appelant est le coffre.
    pub fn agent(tx_cap: u128, daily_cap: u128) -> Self {
        let mut fx = Self::new(tx_cap, daily_cap);
        fx.as_agent();
        fx
    }

    /// Appelant suivant : le propriétaire, avec les 1 yoctoNEAR requis.
    pub fn as_owner(&mut self) -> &mut Self {
        self.context(OWNER, 1)
    }

    /// Appelant suivant : le coffre lui-même (clé d'agent ou clé de gel).
    pub fn as_agent(&mut self) -> &mut Self {
        self.context(VAULT, 0)
    }

    /// Appelant suivant : un compte étranger, sans dépôt.
    pub fn as_stranger(&mut self) -> &mut Self {
        self.context(MALLORY, 0)
    }

    /// Place l'horloge du bloc à `at` nanosecondes.
    pub fn jump(&mut self, at: u64) -> &mut Self {
        self.at = at;
        self.refresh();
        self
    }

    /// Avance l'horloge du bloc.
    pub fn advance(&mut self, ns: u64) -> &mut Self {
        self.jump(self.at + ns)
    }

    /// Fixe le résultat de la promesse sortante, puis renvoie le contexte.
    pub fn promise_result(&mut self, result: PromiseResult) -> &mut Self {
        set_env(&self.caller, self.deposit, self.at, Some(result));
        self
    }

    fn context(&mut self, caller: &str, deposit: u128) -> &mut Self {
        self.caller = caller.to_string();
        self.deposit = deposit;
        self.refresh();
        self
    }

    fn refresh(&mut self) {
        set_env(&self.caller, self.deposit, self.at, None);
    }
}

/// Action de transfert simple.
pub fn transfer(to: &str, amount: u128) -> VaultAction {
    VaultAction::Transfer {
        receiver_id: acct(to),
        amount: U128(amount),
    }
}

/// Action d'appel de fonction sur un contrat, avec dépôt.
pub fn call(to: &str, method: &str, deposit: u128) -> VaultAction {
    VaultAction::Call {
        receiver_id: acct(to),
        method_name: method.to_string(),
        args: "{}".to_string(),
        deposit: U128(deposit),
        gas: Gas::from_tgas(30),
    }
}

/// Exécute une action ; rend le code de refus, ou `None` si elle est passée.
pub fn run(fx: &mut Fixture, action: VaultAction) -> Option<String> {
    attempt(|| {
        let promise = fx.vault.execute(action);
        drop(promise);
    })
}

/// Lance un appel et rend le message de panique, ou `None` s'il est passé.
pub fn attempt<T>(f: impl FnOnce() -> T) -> Option<String> {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(_) => None,
        Err(payload) => Some(panic_message(payload.as_ref())),
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(text) = payload.downcast_ref::<String>() {
        return text.clone();
    }
    match payload.downcast_ref::<&str>() {
        Some(text) => (*text).to_string(),
        None => "panique sans message".to_string(),
    }
}

/// Installe un contexte de bloc : appelant, dépôt joint, date, résultat éventuel.
fn set_env(caller: &str, deposit: u128, at: u64, result: Option<PromiseResult>) {
    let ctx = VMContextBuilder::new()
        .current_account_id(acct(VAULT))
        .predecessor_account_id(acct(caller))
        .signer_account_id(acct(caller))
        .block_timestamp(at)
        .attached_deposit(NearToken::from_yoctonear(deposit))
        .build();
    let results: Vec<PromiseResult> = result.into_iter().collect();
    testing_env!(
        ctx,
        near_sdk::test_vm_config(),
        RuntimeFeesConfig::test(),
        HashMap::default(),
        results
    );
}
