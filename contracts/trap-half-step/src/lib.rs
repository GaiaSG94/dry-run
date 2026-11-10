//! Piège 3 : `half-step.test.near` — encaisse puis fait échouer le reçu suivant.
//!
//! `trap` garde le dépôt, puis lance un appel vers `ghost.test.near`, un compte
//! qui n'existe pas : sur NEAR le reçu échoue **après** que les fonds ont bougé,
//! et rien n'annule la transaction (pas d'atomicité entre reçus).
//!
//! C'est R9 (« un reçu échoue après une sortie de fonds ») qui doit mordre : le
//! simulateur voit le reçu en échec et les fonds coincés chez le piège.
//!
//! Distinct de `drainer` (qui ne fait aucun appel) : ici le piège *essaie* de
//! continuer et se casse en route.

use near_sdk::json_types::U128;
use near_sdk::{env, near, AccountId, Gas, NearToken, Promise};

/// Compte inexistant qui fera échouer le reçu.
pub const GHOST: &str = "ghost.test.near";
/// Gas laissé à l'appel condamné.
const GAS_FOR_GHOST: Gas = Gas::from_tgas(5);

/// État du piège.
#[near(contract_state)]
#[derive(Default)]
pub struct HalfStep {
    /// Fonds encaissés puis abandonnés, en yoctoNEAR.
    stranded: u128,
    /// Nombre d'appels reçus.
    calls: u64,
}

/// Résout un compte de démonstration ; panique proprement si l'identifiant est invalide.
fn account(name: &str) -> AccountId {
    match name.parse() {
        Ok(account) => account,
        Err(_) => env::panic_str("Identifiant de compte invalide"),
    }
}

#[near]
impl HalfStep {
    /// Encaissement suivi d'un appel qui échouera : les fonds restent coincés.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        let deposit = env::attached_deposit().as_yoctonear();
        self.stranded = self.stranded.saturating_add(deposit);
        self.calls = self.calls.saturating_add(1);
        env::log_str(&format!(
            "half-step: {} yoctoNEAR encaisses, puis appel vers {}",
            deposit, GHOST
        ));

        // Deuxième temps du parcours : un compte qui n'existe pas.
        // Le reçu sera marqué « failed » ; l'encaissement, lui, reste acquis.
        Promise::new(account(GHOST))
            .function_call(
                "collect".to_string(),
                args.into_bytes(),
                NearToken::from_yoctonear(0),
                GAS_FOR_GHOST,
            )
            .detach();
        true
    }

    /// Fonds encaissés puis abandonnés, en yoctoNEAR.
    pub fn stranded(&self) -> U128 {
        U128(self.stranded)
    }

    /// Nombre d'appels reçus.
    pub fn calls(&self) -> u64 {
        self.calls
    }

    /// Compte visé par l'appel condamné.
    pub fn ghost(&self) -> AccountId {
        account(GHOST)
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
    fn l_appel_suivant_part_vers_un_compte_inexistant() {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("half-step.test.near"))
            .predecessor_account_id(id("vault.test.near"))
            .signer_account_id(id("vault.test.near"))
            .attached_deposit(NearToken::from_yoctonear(2 * 10u128.pow(24)))
            .build();
        testing_env!(ctx);

        let mut contract = HalfStep::default();
        assert!(contract.trap("{}".to_string()));

        // Le contrat garde le dépôt : aucun transfert de retour n'est créé.
        assert_eq!(contract.stranded(), U128(2 * 10u128.pow(24)));

        let receipts = get_created_receipts();
        assert_eq!(receipts.len(), 1, "un seul reçu sortant : celui du fantôme");
        assert_eq!(receipts[0].receiver_id, id(GHOST));
        assert!(
            receipts[0].actions.iter().any(|a| matches!(
                a,
                MockAction::FunctionCallWeight { method_name, .. } if method_name == b"collect"
            )),
            "le reçu doit appeler collect sur le compte fantôme"
        );
        // Aucun reçu vers le coffre : rien n'est rendu, les fonds restent coincés.
        assert!(
            receipts
                .iter()
                .all(|r| r.receiver_id != id("vault.test.near")),
            "aucun remboursement ne doit etre cree"
        );
    }
}
