//! Piège 6 : `shapeshifter.test.near` — honnête au premier contact, puis remplacé.
//!
//! Premier passage : `trap` rend 1,9 TEST au coffre (comportement honnête, donc
//! ALLOW côté effets). Ensuite, `deploy` remplace le code du **même compte** :
//! l'empreinte change et R6 (« contrat connu dont l'empreinte du code a changé »)
//! mord à la simulation suivante.
//!
//! `deploy` est volontairement **ouvert à tous** : c'est la faille que la démo
//! illustre (un contrat approuvé une fois ne reste pas approuvé).
//!
//! Prérequis : la réserve de TEST est approvisionnée par le propriétaire (`fund`).
//! Remarque de démo : le code déployé par `deploy` doit accepter l'état courant
//! (mêmes champs, même sérialisation JSON), sinon les appels suivants paniqueront
//! à la désérialisation — ce qui est justement visible dans les effets.

use near_sdk::json_types::U128;
use near_sdk::{env, near, serde_json, AccountId, Gas, NearToken, PanicOnDefault, Promise};

/// Une unité de TEST (18 décimales).
const ONE_TEST: u128 = 10u128.pow(18);
/// Montant honnête du premier contact : 1,9 TEST.
const HONEST_OUT: u128 = 19 * 10u128.pow(17);
/// Gas laissé au transfert de jetons.
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(10);

/// État du piège.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Shapeshifter {
    owner_id: AccountId,
    token_id: AccountId,
    /// Réserve de TEST, en unités de base.
    reserve: u128,
    /// Nombre de remplacements de code effectués.
    replacements: u64,
}

#[near]
impl Shapeshifter {
    /// Crée le piège (réserve vide au départ).
    #[init]
    pub fn new(owner_id: AccountId, token_id: AccountId) -> Self {
        Self {
            owner_id,
            token_id,
            reserve: 0,
            replacements: 0,
        }
    }

    /// Premier contact : comportement honnête, 1,9 TEST rendus au coffre.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        let out = HONEST_OUT;
        if out > self.reserve {
            env::panic_str("Reserve insuffisante");
        }
        self.reserve -= out;
        let caller = env::predecessor_account_id();
        env::log_str(&format!(
            "shapeshifter: {} TEST rendus (args: {})",
            out, args
        ));
        let payload = serde_json::json!({
            "receiver_id": caller,
            "amount": U128(out),
            "memo": null,
        });
        let bytes = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(_) => env::panic_str("Arguments de transfert non serialisables"),
        };
        Promise::new(self.token_id.clone())
            .function_call(
                "ft_transfer".to_string(),
                bytes,
                NearToken::from_yoctonear(1),
                GAS_FOR_FT_TRANSFER,
            )
            .detach();
        true
    }

    /// Remplacement du code du compte — **aucun contrôle d'accès**, c'est le piège.
    pub fn deploy(&mut self, code: Vec<u8>) {
        if code.is_empty() {
            env::panic_str("Code vide");
        }
        self.replacements = self.replacements.saturating_add(1);
        env::log_str(&format!(
            "shapeshifter: code remplace ({} octets, remplacement n°{})",
            code.len(),
            self.replacements
        ));
        Promise::new(env::current_account_id())
            .deploy_contract(code)
            .detach();
    }

    /// Réapprovisionnement de la réserve par le propriétaire.
    pub fn fund(&mut self, amount: U128) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire peut approvisionner");
        }
        let value: u128 = amount.into();
        self.reserve = self.reserve.saturating_add(value);
    }

    /// Réserve de TEST disponible.
    pub fn reserve(&self) -> U128 {
        U128(self.reserve)
    }

    /// Montant honnête annoncé pour le premier contact.
    pub fn honest_out(&self) -> U128 {
        U128(HONEST_OUT)
    }

    /// Nombre de remplacements de code.
    pub fn replacements(&self) -> u64 {
        self.replacements
    }

    /// Taille de la réserve exprimée en TEST entiers.
    pub fn reserve_whole(&self) -> u128 {
        self.reserve / ONE_TEST
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
            .current_account_id(id("shapeshifter.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .attached_deposit(NearToken::from_yoctonear(depot))
            .build();
        testing_env!(ctx);
    }

    fn finance() -> Shapeshifter {
        contexte("owner.test.near", 0);
        let mut contract = Shapeshifter::new(id("owner.test.near"), id("test-token.test.near"));
        contract.fund(U128(10 * ONE_TEST));
        contract
    }

    #[test]
    fn premier_contact_honnete() {
        let mut contract = finance();
        contexte("vault.test.near", 2 * 10u128.pow(24));

        assert!(contract.trap("{}".to_string()));
        assert_eq!(contract.reserve(), U128(10 * ONE_TEST - HONEST_OUT));

        let receipts = get_created_receipts();
        let vers_jeton = receipts
            .iter()
            .find(|r| r.receiver_id == id("test-token.test.near"))
            .expect("le jeton doit partir vers le coffre");
        let args = vers_jeton
            .actions
            .iter()
            .find_map(|a| match a {
                MockAction::FunctionCallWeight {
                    method_name, args, ..
                } => Some((method_name.clone(), args.clone())),
                _ => None,
            })
            .expect("un appel ft_transfer doit exister");
        assert_eq!(args.0, b"ft_transfer".to_vec());
        let texte = String::from_utf8_lossy(&args.1);
        assert!(texte.contains("vault.test.near"));
        assert!(texte.contains(&HONEST_OUT.to_string()));
    }

    #[test]
    fn deploy_remplace_le_code_sans_controle() {
        let mut contract = finance();
        // N'importe qui peut appeler deploy : pas de contrôle d'accès.
        contexte("mallory.test.near", 0);

        contract.deploy(vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);

        assert_eq!(contract.replacements(), 1);
        let receipts = get_created_receipts();
        let cible = receipts
            .iter()
            .find(|r| r.receiver_id == id("shapeshifter.test.near"))
            .expect("le deploiement vise le compte lui-meme");
        assert!(cible.actions.iter().any(|a| matches!(
            a,
            MockAction::DeployContract { code, .. } if code.len() == 8
        )));
    }

    #[test]
    #[should_panic(expected = "Seul le proprietaire")]
    fn fund_refuse_hors_proprietaire() {
        let mut contract = finance();
        contexte("mallory.test.near", 0);
        contract.fund(U128(1));
    }
}
