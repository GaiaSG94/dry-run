//! NFT minimal (NEP-171 réduit) pour la démonstration.
//!
//! Compte déployé : `test-nft.test.near`. Émetteur : `owner.test.near`.
//! Un seul jeton est utilisé dans la démo : `token_id = "1"`.
//!
//! Règle volontairement permissive pour la démo : **le détenteur courant** du jeton
//! peut approuver un tiers (`nft_approve`). C'est ce qui permet à `trap-approver`
//! de faire approuver `mallory.test.near` quand le coffre détient le jeton.
//!
//! Les événements sont publiés au format NEP-297 / NEP-171 sur une seule ligne :
//! `EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_approve","data":[...]}`
//! ce qui permet au simulateur (`dryrun-sim`) de les reconnaître.

use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, serde_json, AccountId, PanicOnDefault};

/// Version du standard annoncée dans les événements.
const NFT_STANDARD: &str = "nep171";
const NFT_VERSION: &str = "1.0.0";

/// État de la collection.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TestNft {
    owner_id: AccountId,
    /// Jeton → détenteur courant.
    owners: LookupMap<String, AccountId>,
    /// (jeton, compte approuvé) → approuvé ou non.
    approvals: LookupMap<(String, AccountId), bool>,
}

/// Publie un événement NEP-297 sur une seule ligne.
fn emit(event: &str, data: serde_json::Value) {
    let payload = serde_json::json!({
        "standard": NFT_STANDARD,
        "version": NFT_VERSION,
        "event": event,
        "data": data,
    });
    match serde_json::to_string(&payload) {
        Ok(line) => env::log_str(&format!("EVENT_JSON:{}", line)),
        Err(_) => env::log_str("EVENT_JSON:{}"),
    }
}

#[near]
impl TestNft {
    /// Crée la collection. `owner_id` est le seul compte autorisé à émettre.
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            owners: LookupMap::new(b"o"),
            approvals: LookupMap::new(b"p"),
        }
    }

    /// Émission d'un jeton, réservée à l'émetteur.
    pub fn mint_to(&mut self, to: AccountId, token_id: String) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul l'emetteur peut frapper un jeton");
        }
        if self.owners.contains_key(&token_id) {
            env::panic_str("Jeton deja frappe");
        }
        self.owners.insert(token_id.clone(), to.clone());
        emit(
            "nft_mint",
            serde_json::json!([{ "token_id": token_id, "owner_id": to }]),
        );
    }

    /// Transfert du jeton par son détenteur courant.
    #[payable]
    pub fn nft_transfer(&mut self, to: AccountId, token_id: String) {
        let caller = env::predecessor_account_id();
        let holder = self.holder(&token_id);
        if holder != caller {
            env::panic_str("Seul le detenteur peut transferer ce jeton");
        }
        self.owners.insert(token_id.clone(), to.clone());
        emit(
            "nft_transfer",
            serde_json::json!([{ "token_id": token_id, "old_owner_id": holder, "new_owner_id": to }]),
        );
    }

    /// Approbation d'un tiers par le détenteur courant (1 yocto de courtoisie accepté).
    #[payable]
    pub fn nft_approve(&mut self, to: AccountId, token_id: String) {
        let caller = env::predecessor_account_id();
        let holder = self.holder(&token_id);
        if holder != caller {
            env::panic_str("Seul le detenteur peut approuver ce jeton");
        }
        self.approvals.insert((token_id.clone(), to.clone()), true);
        emit(
            "nft_approve",
            serde_json::json!([{
                "token_id": token_id,
                "owner_id": holder,
                "approved_account_id": to,
            }]),
        );
    }

    /// Révoque une approbation.
    pub fn nft_revoke(&mut self, to: AccountId, token_id: String) {
        if self.holder(&token_id) != env::predecessor_account_id() {
            env::panic_str("Seul le detenteur peut revoquer une approbation");
        }
        self.approvals.insert((token_id, to), false);
    }

    /// Le compte `to` est-il approuvé pour ce jeton ?
    pub fn nft_is_approved(&self, to: AccountId, token_id: String) -> bool {
        self.approvals
            .get(&(token_id, to))
            .copied()
            .unwrap_or(false)
    }

    /// Détenteur courant d'un jeton.
    pub fn nft_owner_of(&self, token_id: String) -> AccountId {
        self.holder(&token_id)
    }

    /// Émetteur de la collection.
    pub fn nft_owner_id(&self) -> AccountId {
        self.owner_id.clone()
    }

    /// Métadonnées NEP-171 minimales.
    pub fn nft_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "spec": "nft-1.1.0",
            "name": "Dry Run NFT",
            "symbol": "DRNFT",
            "decimals": 0,
            "base_uri": "",
            "icon": null,
            "reference": null,
            "reference_hash": null,
            "royalty": U128(0),
        })
    }

    /// Lecture du détenteur, avec panique si le jeton n'existe pas.
    fn holder(&self, token_id: &str) -> AccountId {
        match self.owners.get(token_id).cloned() {
            Some(account) => account,
            None => env::panic_str("Jeton inconnu"),
        }
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
            .current_account_id(id("test-nft.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .build();
        testing_env!(ctx);
    }

    /// Collection avec le jeton `1` détenu par `vault.test.near`.
    fn collection() -> TestNft {
        contexte("owner.test.near");
        let mut nft = TestNft::new(id("owner.test.near"));
        nft.mint_to(id("vault.test.near"), "1".to_string());
        nft
    }

    #[test]
    fn frappe_et_transfert() {
        let mut nft = collection();
        assert_eq!(nft.nft_owner_of("1".to_string()), id("vault.test.near"));

        contexte("vault.test.near");
        nft.nft_transfer(id("bob.test.near"), "1".to_string());
        assert_eq!(nft.nft_owner_of("1".to_string()), id("bob.test.near"));
    }

    #[test]
    fn le_detenteur_approuve_un_tiers() {
        let mut nft = collection();
        contexte("vault.test.near");
        assert!(!nft.nft_is_approved(id("mallory.test.near"), "1".to_string()));
        nft.nft_approve(id("mallory.test.near"), "1".to_string());
        assert!(nft.nft_is_approved(id("mallory.test.near"), "1".to_string()));
    }

    #[test]
    #[should_panic(expected = "Seul le detenteur peut approuver")]
    fn approbation_refusee_hors_detenteur() {
        let mut nft = collection();
        contexte("mallory.test.near");
        nft.nft_approve(id("mallory.test.near"), "1".to_string());
    }

    #[test]
    #[should_panic(expected = "Seul l'emetteur")]
    fn frappe_refusee_hors_emetteur() {
        let mut nft = collection();
        contexte("mallory.test.near");
        nft.mint_to(id("mallory.test.near"), "2".to_string());
    }
}
