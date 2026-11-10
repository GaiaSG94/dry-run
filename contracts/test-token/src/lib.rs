//! Jeton fongible minimal à la NEP-141 : jeton `TEST` de la démonstration.
//!
//! Le jeton n'est pas complet (pas de dépôt de stockage, pas de liste blanche) :
//! il expose exactement les méthodes utilisées par la démo et par le coffre.
//!
//! Compte déployé : `test-token.test.near`. Propriétaire du jeton : `owner.test.near`.

use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{
    env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
    PromiseOrValue,
};

/// Gas laissé au contrat appelé par `ft_transfer_call` pour son `ft_on_transfer`.
const GAS_FOR_FT_ON_TRANSFER: Gas = Gas::from_tgas(10);
/// Gas laissé au rappel `ft_resolve_transfer`.
const GAS_FOR_RESOLVE: Gas = Gas::from_tgas(5);
/// Décimales du jeton : 18, comme NEAR.
pub const DECIMALS: u8 = 18;

/// Métadonnées NEP-141.
#[near(serializers = [json])]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

/// État du jeton.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TestToken {
    owner_id: AccountId,
    accounts: LookupMap<AccountId, u128>,
    total_supply: u128,
}

/// Sérialise des arguments JSON ; panique proprement en cas d'échec.
fn json_args<T: near_sdk::serde::Serialize>(value: &T) -> Vec<u8> {
    match serde_json::to_vec(value) {
        Ok(bytes) => bytes,
        Err(_) => env::panic_str("Arguments non sérialisables en JSON"),
    }
}

#[near]
impl TestToken {
    /// Crée le jeton. `owner_id` est le seul compte autorisé à émettre.
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            accounts: LookupMap::new(b"a"),
            total_supply: 0,
        }
    }

    /// Émission de jetons, réservée au propriétaire du jeton.
    pub fn mint_to(&mut self, account_id: AccountId, amount: U128) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire du jeton peut emettre");
        }
        let value: u128 = amount.into();
        let current = self.balance_of(&account_id);
        self.accounts
            .insert(account_id, current.saturating_add(value));
        self.total_supply = self.total_supply.saturating_add(value);
    }

    /// Solde d'un compte.
    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(self.balance_of(&account_id))
    }

    /// Masse monétaire en circulation.
    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }

    /// Métadonnées du jeton (symbole `TEST`).
    pub fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "Dry Run Test Token".to_string(),
            symbol: "TEST".to_string(),
            decimals: DECIMALS,
        }
    }

    /// Propriétaire du jeton (seul compte autorisé à émettre).
    pub fn owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Transfert simple de jetons (dépôt de courtoisie accepté, ignoré).
    ///
    /// Signature volontairement minimale : `ft_transfer(receiver_id, amount)`.
    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128) {
        let sender = env::predecessor_account_id();
        self.move_tokens(&sender, &receiver_id, amount.into());
    }

    /// Transfert avec appel : le contrat reçoit les jetons puis `ft_on_transfer`.
    ///
    /// Le rappel `ft_resolve_transfer` rembourse l'émetteur si le récepteur rend
    /// une partie des jetons (valeur de retour de `ft_on_transfer`).
    #[payable]
    pub fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        let sender = env::predecessor_account_id();
        self.move_tokens(&sender, &receiver_id, amount.into());

        let on_transfer = Promise::new(receiver_id.clone()).function_call(
            "ft_on_transfer".to_string(),
            json_args(&serde_json::json!({
                "sender_id": sender,
                "amount": amount,
                "msg": msg,
            })),
            NearToken::from_yoctonear(0),
            GAS_FOR_FT_ON_TRANSFER,
        );
        let resolve = Self::ext_self()
            .with_static_gas(GAS_FOR_RESOLVE)
            .ft_resolve_transfer(sender, receiver_id, amount);
        PromiseOrValue::Promise(on_transfer.then(resolve))
    }

    /// Rappel privé de `ft_transfer_call` : crédite l'émetteur du montant non consommé.
    #[private]
    pub fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
        #[callback_result] unused: Result<U128, PromiseError>,
    ) -> U128 {
        let sent: u128 = amount.into();
        let unused: u128 = match unused {
            Ok(U128(value)) => value.min(sent),
            // Le récepteur a échoué : tout est rendu.
            Err(_) => sent,
        };
        let kept = sent.saturating_sub(unused);
        if unused > 0 {
            let held_value = self.balance_of(&receiver_id);
            let held = held_value.min(unused);
            self.accounts.insert(receiver_id, held_value - held);
            let back = self.balance_of(&sender_id).saturating_add(held);
            self.accounts.insert(sender_id, back);
            return U128(sent.saturating_sub(held));
        }
        U128(kept)
    }

    /// Hook NEP-141 de démonstration : ce jeton n'appelle jamais lui-même un récepteur,
    /// la méthode existe pour que les appels de la démo aient une cible valide.
    /// Retourne 0 : tout est accepté.
    pub fn ft_on_transfer(&self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        env::log_str(&format!(
            "ft_on_transfer: {} a envoye {} (msg: {})",
            sender_id,
            u128::from(amount),
            msg
        ));
        U128(0)
    }

    /// Lecture interne du solde.
    fn balance_of(&self, account_id: &AccountId) -> u128 {
        self.accounts.get(account_id).copied().unwrap_or(0)
    }

    /// Déplacement interne de jetons, avec contrôle de solde.
    fn move_tokens(&mut self, sender: &AccountId, receiver: &AccountId, amount: u128) {
        let from = self.balance_of(sender);
        if from < amount {
            env::panic_str("Solde insuffisant");
        }
        let to = self.balance_of(receiver);
        self.accounts.insert(sender.clone(), from - amount);
        self.accounts
            .insert(receiver.clone(), to.saturating_add(amount));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn id(name: &str) -> AccountId {
        match name.parse() {
            Ok(a) => a,
            Err(_) => panic!("compte de test invalide"),
        }
    }

    fn contexte(signer: &str) {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("test-token.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .build();
        testing_env!(ctx);
    }

    fn jeton() -> TestToken {
        contexte("owner.test.near");
        let mut contract = TestToken::new(id("owner.test.near"));
        contract.mint_to(id("owner.test.near"), U128(1_000));
        contract
    }

    #[test]
    fn emission_et_transfert() {
        let mut contract = jeton();
        assert_eq!(contract.ft_balance_of(id("owner.test.near")), U128(1_000));

        contexte("owner.test.near");
        contract.ft_transfer(id("vault.test.near"), U128(250));

        assert_eq!(contract.ft_balance_of(id("owner.test.near")), U128(750));
        assert_eq!(contract.ft_balance_of(id("vault.test.near")), U128(250));
        assert_eq!(contract.ft_total_supply(), U128(1_000));
        assert_eq!(contract.ft_metadata().symbol, "TEST");
    }

    #[test]
    #[should_panic(expected = "Seul le proprietaire")]
    fn emission_refusee_hors_proprietaire() {
        let mut contract = jeton();
        contexte("mallory.test.near");
        contract.mint_to(id("mallory.test.near"), U128(1));
    }

    #[test]
    #[should_panic(expected = "Solde insuffisant")]
    fn transfert_sans_solde_refuse() {
        let mut contract = jeton();
        contexte("mallory.test.near");
        contract.ft_transfer(id("bob.test.near"), U128(1));
    }
}
