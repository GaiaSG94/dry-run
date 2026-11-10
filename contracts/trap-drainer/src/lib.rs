//! Piège 1 : `drainer.test.near` — encaisse le dépôt et ne rend **rien**.
//!
//! Appelé par le coffre via `VaultAction::Call { method_name: "trap", deposit: 2 NEAR }`.
//! Le contrat répond « succès », garde les fonds et ne crée aucun reçu sortant :
//! c'est R8 (« reçu moins que le minimum déclaré, ou rien ») qui doit mordre.

use near_sdk::json_types::U128;
use near_sdk::{env, near};

/// État du piège.
#[near(contract_state)]
#[derive(Default)]
pub struct Drainer {
    /// Total des dépôts encaissés, en yoctoNEAR (pour la démonstration).
    kept: u128,
    /// Nombre d'appels reçus.
    calls: u64,
}

#[near]
impl Drainer {
    /// Encaissement sans contrepartie : le dépôt reste sur le contrat.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        let deposit = env::attached_deposit().as_yoctonear();
        self.kept = self.kept.saturating_add(deposit);
        self.calls = self.calls.saturating_add(1);
        env::log_str(&format!(
            "drainer: {} yoctoNEAR encaisses, aucun transfert en retour (args: {})",
            deposit, args
        ));
        // Mensonge assumé : on répond « vrai » sans rien rendre.
        true
    }

    /// Total encaissé, en yoctoNEAR.
    pub fn kept(&self) -> U128 {
        U128(self.kept)
    }

    /// Nombre d'appels reçus.
    pub fn calls(&self) -> u64 {
        self.calls
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_created_receipts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::NearToken;

    fn id(name: &str) -> near_sdk::AccountId {
        match name.parse() {
            Ok(a) => a,
            Err(_) => panic!("compte de test invalide"),
        }
    }

    fn contexte(signer: &str, depot: u128) {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("drainer.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .attached_deposit(NearToken::from_yoctonear(depot))
            .build();
        testing_env!(ctx);
    }

    #[test]
    fn le_depot_est_garde_et_rien_n_est_rendu() {
        let mut drainer = Drainer::default();
        contexte("vault.test.near", 2 * 10u128.pow(24));

        let ok = drainer.trap("{}".to_string());

        assert!(ok, "le piège repond toujours vrai");
        assert_eq!(drainer.kept(), U128(2 * 10u128.pow(24)));
        assert_eq!(drainer.calls(), 1);
        assert!(
            get_created_receipts().is_empty(),
            "aucun reçu ne doit etre cree : rien n'est rendu au coffre"
        );
    }
}
