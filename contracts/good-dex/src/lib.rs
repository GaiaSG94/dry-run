//! Échange honnête de référence : `good-dex.test.near`.
//!
//! Le coffre dépose 2 NEAR et reçoit `min_out` unités de `TEST`, puis 0,02 NEAR de
//! frais lui sont rendus. Le contrat paie le jeton depuis sa propre réserve de TEST.
//!
//! Prérequis de la démo (voir `docs/SPEC.md` §3) :
//! - `test-token.test.near` est déployé et le jeton `TEST` est frappé par `owner.test.near` ;
//! - `owner.test.near` approvisionne la réserve du DEX, au choix :
//!   1. par `ft_transfer_call(receiver_id = "good-dex.test.near", amount, msg = "fund")`
//!      appelé sur `test-token` → `ft_on_transfer` crédite la réserve (chemin recommandé) ;
//!   2. ou par `ft_transfer` vers `good-dex.test.near` **puis** `fund(amount)` sur le DEX.
//!
//!   Ne jamais combiner les deux pour la même quantité (le crédit serait doublé).

use near_sdk::json_types::U128;
use near_sdk::{env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise};

/// Prix d'un échange, en yoctoNEAR (2 NEAR).
pub const PRICE: u128 = 2 * 10u128.pow(24);
/// Frais rendus à l'acheteur, en yoctoNEAR (0,02 NEAR).
pub const FEE_REFUND: u128 = 2 * 10u128.pow(22);
/// Symbole du seul jeton échangé.
pub const SYMBOL: &str = "TEST";
/// Gas laissé au transfert de jetons.
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(10);

/// État du DEX.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct GoodDex {
    owner_id: AccountId,
    /// Compte du contrat `test-token`.
    token_id: AccountId,
    /// Réserve interne de TEST, en unités de base (18 décimales).
    reserve: u128,
}

#[near]
impl GoodDex {
    /// Crée le DEX pour le jeton `token_id`.
    #[init]
    pub fn new(owner_id: AccountId, token_id: AccountId) -> Self {
        Self {
            owner_id,
            token_id,
            reserve: 0,
        }
    }

    /// Échange honnête : 2 NEAR contre `min_out` TEST, frais rendus.
    ///
    /// Retourne `true` : l'appel a bien produit le transfert et le remboursement.
    #[payable]
    pub fn swap(&mut self, token_out: String, min_out: U128) -> bool {
        if token_out != SYMBOL {
            env::panic_str("Jeton non pris en charge : seul TEST est echange");
        }
        let deposit = env::attached_deposit();
        if deposit.as_yoctonear() < PRICE {
            env::panic_str("Depot insuffisant : 2 NEAR attendus");
        }
        let out: u128 = min_out.into();
        if out == 0 {
            env::panic_str("min_out doit etre strictement positif");
        }
        if out > self.reserve {
            env::panic_str("Reserve du DEX insuffisante");
        }
        self.reserve -= out;

        let buyer = env::predecessor_account_id();
        // 1. Le jeton part vers l'acheteur (le coffre).
        self.pay_test(&buyer, out);
        // 2. Les frais lui sont rendus : sortie nette = 1,98 NEAR.
        Promise::new(buyer)
            .transfer(NearToken::from_yoctonear(FEE_REFUND))
            .detach();
        env::log_str(&format!("swap: {} TEST pour {} yoctoNEAR", out, PRICE));
        true
    }

    /// Réapprovisionnement de la réserve par le propriétaire (voir l'entête du fichier).
    pub fn fund(&mut self, amount: U128) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire peut approvisionner le DEX");
        }
        let value: u128 = amount.into();
        self.reserve = self.reserve.saturating_add(value);
        env::log_str(&format!("fund: reserve = {} TEST", self.reserve));
    }

    /// Réception NEP-141 : le jeton est accepté et crédité à la réserve.
    /// Retourne 0 (rien à rendre) : l'échange est honnête.
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        let value: u128 = amount.into();
        self.reserve = self.reserve.saturating_add(value);
        env::log_str(&format!(
            "ft_on_transfer: {} a credite {} TEST (msg: {})",
            sender_id, value, msg
        ));
        U128(0)
    }

    /// Réserve disponible, en unités de base.
    pub fn reserve(&self) -> U128 {
        U128(self.reserve)
    }

    /// Compte du jeton échangé.
    pub fn token_id(&self) -> AccountId {
        self.token_id.clone()
    }

    /// Propriétaire du DEX.
    pub fn owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Envoie `amount` TEST à `to` depuis la réserve, via `test-token`.
    fn pay_test(&self, to: &AccountId, amount: u128) {
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
                // 1 yoctoNEAR : exigence NEP-141 de courtoisie.
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
            .current_account_id(id("good-dex.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .attached_deposit(NearToken::from_yoctonear(depot))
            .build();
        testing_env!(ctx);
    }

    /// 10 TEST de réserve.
    fn dex_approvisionne() -> GoodDex {
        contexte("owner.test.near", 0);
        let mut dex = GoodDex::new(id("owner.test.near"), id("test-token.test.near"));
        dex.fund(U128(10 * 10u128.pow(18)));
        dex
    }

    #[test]
    fn swap_rend_le_jeton_et_les_frais() {
        let mut dex = dex_approvisionne();
        let demande = 19 * 10u128.pow(17); // 1,9 TEST
        contexte("vault.test.near", PRICE);

        let ok = dex.swap(SYMBOL.to_string(), U128(demande));

        assert!(ok, "le swap honnete doit reussir");
        assert_eq!(dex.reserve(), U128(10 * 10u128.pow(18) - demande));

        let receipts = get_created_receipts();
        let vers_jeton = receipts
            .iter()
            .find(|r| r.receiver_id == id("test-token.test.near"))
            .expect("un transfert de jetons doit etre cree");
        assert!(vers_jeton.actions.iter().any(|a| matches!(
            a,
            MockAction::FunctionCallWeight { method_name, .. } if method_name == b"ft_transfer"
        )));

        let frais = receipts
            .iter()
            .find(|r| r.receiver_id == id("vault.test.near"))
            .expect("les frais doivent etre rendus au coffre");
        assert!(frais.actions.iter().any(|a| matches!(
            a,
            MockAction::Transfer { deposit, .. } if deposit.as_yoctonear() == FEE_REFUND
        )));
    }

    #[test]
    #[should_panic(expected = "Depot insuffisant")]
    fn swap_refuse_sans_depot() {
        let mut dex = dex_approvisionne();
        contexte("vault.test.near", 0);
        dex.swap(SYMBOL.to_string(), U128(1));
    }

    #[test]
    #[should_panic(expected = "Reserve du DEX insuffisante")]
    fn swap_refuse_si_reserve_trop_basse() {
        let mut dex = dex_approvisionne();
        contexte("vault.test.near", PRICE);
        dex.swap(SYMBOL.to_string(), U128(11 * 10u128.pow(18)));
    }

    #[test]
    fn ft_on_transfer_credite_la_reserve() {
        let mut dex = dex_approvisionne();
        contexte("owner.test.near", 0);
        let rendu = dex.ft_on_transfer(id("owner.test.near"), U128(5), "fund".to_string());
        assert_eq!(rendu, U128(0), "le DEX accepte tout");
        assert_eq!(dex.reserve(), U128(10 * 10u128.pow(18) + 5));
    }

    #[test]
    #[should_panic(expected = "Seul le proprietaire")]
    fn fund_refuse_hors_proprietaire() {
        let mut dex = dex_approvisionne();
        contexte("mallory.test.near", 0);
        dex.fund(U128(1));
    }
}
