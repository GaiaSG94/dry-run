//! Piège 4 : `liar.test.near` — répond « succès » sans rien transférer.
//!
//! Le contrat publie même un événement NEP-297 annonçant un échange réussi
//! (1,9 TEST) alors qu'aucun reçu sortant n'est créé. Un pare-feu qui lit les
//! **logs** est trompé ; un pare-feu qui lit les **effets** voit R8.
//!
//! Distinct de `drainer` : `liar` ne comptabilise rien, il ment par le texte.

use near_sdk::json_types::U128;
use near_sdk::{env, near, serde_json};

/// Montant annoncé dans l'événement mensonger (1,9 TEST).
const CLAIMED_OUT: u128 = 19 * 10u128.pow(17);

/// État du piège.
#[near(contract_state)]
#[derive(Default)]
pub struct Liar {
    /// Nombre de mensonges proférés.
    lies: u64,
}

#[near]
impl Liar {
    /// Réponse « succès » sans aucun transfert.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        self.lies = self.lies.saturating_add(1);
        let annonce = serde_json::json!({
            "standard": "nep297",
            "version": "1.0.0",
            "event": "swap",
            "data": [{ "token_out": "TEST", "amount_out": U128(CLAIMED_OUT), "args": args }],
        });
        match serde_json::to_string(&annonce) {
            Ok(ligne) => env::log_str(&format!("EVENT_JSON:{}", ligne)),
            Err(_) => env::log_str("EVENT_JSON:{}"),
        }
        env::log_str("liar: echange reussi (mensonge : aucun jeton n'a bouge)");
        true
    }

    /// Montant prétendument versé.
    pub fn claimed_out(&self) -> U128 {
        U128(CLAIMED_OUT)
    }

    /// Nombre de messages mensongers.
    pub fn lies(&self) -> u64 {
        self.lies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{get_created_receipts, get_logs, VMContextBuilder};
    use near_sdk::{testing_env, NearToken};

    fn id(name: &str) -> near_sdk::AccountId {
        match name.parse() {
            Ok(a) => a,
            Err(_) => panic!("compte de test invalide"),
        }
    }

    #[test]
    fn repond_vrai_sans_creer_de_recu() {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("liar.test.near"))
            .predecessor_account_id(id("vault.test.near"))
            .signer_account_id(id("vault.test.near"))
            .attached_deposit(NearToken::from_yoctonear(2 * 10u128.pow(24)))
            .build();
        testing_env!(ctx);

        let mut liar = Liar::default();
        let ok = liar.trap("{}".to_string());

        assert!(ok, "le piege annonce un succes");
        assert_eq!(liar.lies(), 1, "un mensonge de plus");
        assert!(
            get_created_receipts().is_empty(),
            "aucun transfert : le coffre ne recoit rien (R8)"
        );
        let logs = get_logs();
        assert!(
            logs.iter().any(|l| l.contains("EVENT_JSON") && l.contains("swap")),
            "le contrat publie un evenement mensonger"
        );
    }
}
