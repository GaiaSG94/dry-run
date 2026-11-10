//! Piège 2 : `skimmer.test.near` — rétro-commission de 20 % vers un tiers.
//!
//! `trap` rend 1,6 TEST au coffre mais verse 0,4 TEST à `skim_to` (20 %).
//! Le coffre reçoit donc moins que le minimum annoncé : R7 (bénéficiaire non
//! déclaré) **et** R8 (moins reçu que promis) mordent.
//!
//! La réserve de TEST est approvisionnée par le propriétaire (`fund`), comme
//! pour `good-dex`.
//!
//! Distinct de `good-dex` : le partage 80/20 est câblé, et un second
//! bénéficiaire apparaît dans les effets.

use near_sdk::json_types::U128;
use near_sdk::{env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise};

/// Une unité de TEST (18 décimales).
const ONE_TEST: u128 = 10u128.pow(18);
/// Total distribué à chaque appel : 2 TEST.
const TOTAL: u128 = 2 * ONE_TEST;
/// Part détournée : 20 % soit 0,4 TEST.
const SKIM: u128 = TOTAL / 5;
/// Part rendue au coffre : 1,6 TEST.
const PAYOUT: u128 = TOTAL - SKIM;
/// Gas laissé à chaque transfert de jetons.
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(8);

/// État du piège.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Skimmer {
    owner_id: AccountId,
    token_id: AccountId,
    /// Compte qui touche la rétro-commission (modifiable par le propriétaire).
    skim_to: AccountId,
    /// Réserve de TEST, en unités de base.
    reserve: u128,
}

#[near]
impl Skimmer {
    /// Crée le piège avec son compte de commission.
    #[init]
    pub fn new(owner_id: AccountId, token_id: AccountId, skim_to: AccountId) -> Self {
        Self {
            owner_id,
            token_id,
            skim_to,
            reserve: 0,
        }
    }

    /// Échange « généreux » : 2 TEST annoncés, 1,6 rendus, 0,4 détournés.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        let total = TOTAL;
        if total > self.reserve {
            env::panic_str("Reserve insuffisante");
        }
        self.reserve -= total;

        let caller = env::predecessor_account_id();
        // 1. La commission part discrètement vers le tiers.
        self.pay(&self.skim_to.clone(), SKIM);
        // 2. Le reste est rendu à l'appelant (le coffre).
        self.pay(&caller, PAYOUT);

        env::log_str(&format!(
            "skimmer: {} TEST rendus, {} TEST detournes vers {} (args: {})",
            PAYOUT, SKIM, self.skim_to, args
        ));
        true
    }

    /// Réapprovisionnement par le propriétaire.
    pub fn fund(&mut self, amount: U128) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire peut approvisionner");
        }
        let value: u128 = amount.into();
        self.reserve = self.reserve.saturating_add(value);
    }

    /// Change le compte de commission (propriétaire seulement).
    pub fn set_skim_to(&mut self, account_id: AccountId) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire peut changer le beneficiaire");
        }
        self.skim_to = account_id;
    }

    /// Compte de commission courant.
    pub fn skim_to(&self) -> AccountId {
        self.skim_to.clone()
    }

    /// Réserve de TEST disponible.
    pub fn reserve(&self) -> U128 {
        U128(self.reserve)
    }

    /// Montants annoncés par le piège : (rendu, détourné).
    pub fn split(&self) -> (U128, U128) {
        (U128(PAYOUT), U128(SKIM))
    }

    /// Transfert de `amount` TEST depuis la réserve.
    fn pay(&self, to: &AccountId, amount: u128) {
        let args = serde_json::json!({
            "receiver_id": to,
            "amount": U128(amount),
            "memo": null,
        });
        let payload = match serde_json::to_vec(&args) {
            Ok(bytes) => bytes,
            Err(_) => env::panic_str("Arguments de transfert non serialisables"),
        };
        Promise::new(self.token_id.clone())
            .function_call(
                "ft_transfer".to_string(),
                payload,
                NearToken::from_yoctonear(1),
                GAS_FOR_FT_TRANSFER,
            )
            .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::mock::MockAction;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::testing_env;

    fn id(name: &str) -> AccountId {
        match name.parse() {
            Ok(a) => a,
            Err(_) => panic!("compte de test invalide"),
        }
    }

    fn contexte(signer: &str, depot: u128) {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("skimmer.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .attached_deposit(NearToken::from_yoctonear(depot))
            .build();
        testing_env!(ctx);
    }

    fn skimmer() -> Skimmer {
        contexte("owner.test.near", 0);
        let mut contract = Skimmer::new(
            id("owner.test.near"),
            id("test-token.test.near"),
            id("mallory.test.near"),
        );
        contract.fund(U128(10 * ONE_TEST));
        contract
    }

    /// Destinataires et montants des transferts de jetons demandés.
    fn transferts() -> Vec<(String, u128)> {
        let mut out = Vec::new();
        for receipt in get_created_receipts() {
            for action in receipt.actions {
                if let MockAction::FunctionCallWeight {
                    method_name, args, ..
                } = action
                {
                    if method_name != b"ft_transfer" {
                        continue;
                    }
                    let parsed: serde_json::Value =
                        serde_json::from_slice(&args).unwrap_or(serde_json::Value::Null);
                    let to = parsed["receiver_id"].as_str().unwrap_or("").to_string();
                    let montant = parsed["amount"].as_str().unwrap_or("0").parse().unwrap_or(0);
                    out.push((to, montant));
                }
            }
        }
        out
    }

    #[test]
    fn vingt_pour_cent_partent_chez_mallory() {
        let mut contract = skimmer();
        contexte("vault.test.near", 2 * 10u128.pow(24));

        assert!(contract.trap("{}".to_string()));

        assert_eq!(contract.reserve(), U128(10 * ONE_TEST - TOTAL));
        let versements = transferts();
        assert!(
            versements.contains(&("mallory.test.near".to_string(), SKIM)),
            "mallory doit toucher 0,4 TEST"
        );
        assert!(
            versements.contains(&("vault.test.near".to_string(), PAYOUT)),
            "le coffre ne recoit que 1,6 TEST"
        );
        assert_eq!(SKIM * 5, TOTAL, "la commission vaut 20 % du total");
    }

    #[test]
    fn le_proprietaire_change_le_beneficiaire() {
        let mut contract = skimmer();
        contexte("owner.test.near", 0);
        contract.set_skim_to(id("bob.test.near"));
        assert_eq!(contract.skim_to(), id("bob.test.near"));
    }

    #[test]
    #[should_panic(expected = "Seul le proprietaire")]
    fn changement_de_beneficiaire_refuse() {
        let mut contract = skimmer();
        contexte("mallory.test.near", 0);
        contract.set_skim_to(id("mallory.test.near"));
    }
}
