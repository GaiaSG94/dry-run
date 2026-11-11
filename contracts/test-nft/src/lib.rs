//! Contrat NFT minimal pour la démonstration (`test-nft.test.near`).
//!
//! Un seul jeton est utilisé : `token_id = "1"`. L'émetteur (`owner_id`) frappe
//! le jeton et le transfère au coffre. Ensuite, **tout compte** peut demander
//! une approbation sur ce jeton : c'est volontairement permissif, car c'est
//! exactement la surface d'attaque que le piège `approver` exploite et que la
//! règle R3 du pare-feu doit intercepter.
//!
//! Les événements sont publiés sur une ligne lisible par le simulateur :
//! `nft_approve collection:<compte> token_id:<id> to:<compte>`.

use near_sdk::store::LookupMap;
use near_sdk::{env, near, AccountId, PanicOnDefault};

/// Comptes dont l'approbation peut être révoquée (liste courte de démo).
const APPROVAL_CANDIDATES: &[&str] = &[
    "mallory.test.near",
    "bob.test.near",
    "approver.test.near",
    "vault.test.near",
    "owner.test.near",
];

/// Version du standard annoncée dans les métadonnées.
const NFT_STANDARD: &str = "nep171";
const NFT_VERSION: &str = "1.0.0";

/// État de la collection.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TestNft {
    /// Compte autorisé à frapper des jetons.
    owner_id: AccountId,
    /// Propriétaire courant de chaque jeton.
    owners: LookupMap<String, AccountId>,
    /// Approbations en cours : (jeton, compte) → vrai.
    approvals: LookupMap<(String, AccountId), bool>,
}

/// Métadonnées NEP-171.
#[near(serializers = [json])]
pub struct NftMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
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
        self.owners.insert(token_id.clone(), to.clone());
        env::log_str(&format!(
            "nft_transfer collection:{} token_id:{} to:{}",
            env::current_account_id(),
            token_id,
            to
        ));
    }

    /// Transfert d'un jeton par son détenteur courant (1 yocto de courtoisie).
    #[payable]
    pub fn nft_transfer(&mut self, to: AccountId, token_id: String) {
        let caller = env::predecessor_account_id();
        let holder = self.holder(&token_id);
        if holder != caller {
            env::panic_str("Seul le detenteur peut transferer ce jeton");
        }
        self.owners.insert(token_id.clone(), to.clone());
        // Un transfert révoque les approbations en cours.
        self.clear_approvals(&token_id);
        env::log_str(&format!(
            "nft_transfer collection:{} token_id:{} to:{}",
            env::current_account_id(),
            token_id,
            to
        ));
    }

    /// Approbation d'un tiers.
    ///
    /// Volontairement permissif pour la démonstration : le détenteur **ou**
    /// n'importe quel compte peut demander une approbation. C'est la surface
    /// d'attaque que le piège `approver` exploite et que R3 doit intercepter.
    #[payable]
    pub fn nft_approve(&mut self, to: AccountId, token_id: String) {
        if to == env::predecessor_account_id() {
            // S'approuver soi-même n'a pas de sens : on ignore.
            return;
        }
        self.approvals.insert((token_id.clone(), to.clone()), true);
        env::log_str(&format!(
            "nft_approve collection:{} token_id:{} to:{}",
            env::current_account_id(),
            token_id,
            to
        ));
    }

    /// Révoque une approbation (détenteur seulement).
    pub fn nft_revoke(&mut self, to: AccountId, token_id: String) {
        if self.holder(&token_id) != env::predecessor_account_id() {
            env::panic_str("Seul le detenteur peut revoquer");
        }
        self.approvals.remove(&(token_id, to));
    }

    /// Le compte est-il approuvé pour ce jeton ?
    pub fn nft_is_approved(&self, to: AccountId, token_id: String) -> bool {
        self.approvals
            .get(&(token_id, to))
            .copied()
            .unwrap_or(false)
    }

    /// Détenteur courant du jeton.
    pub fn nft_owner_of(&self, token_id: String) -> AccountId {
        self.holder(&token_id)
    }

    /// Métadonnées de la collection.
    pub fn nft_metadata(&self) -> NftMetadata {
        NftMetadata {
            spec: format!("{NFT_STANDARD}-{NFT_VERSION}"),
            name: "Dry Run Demo NFT".to_string(),
            symbol: "DRYNFT".to_string(),
        }
    }
}

impl TestNft {
    /// Détenteur courant, ou panique si le jeton n'existe pas.
    fn holder(&self, token_id: &str) -> AccountId {
        match self.owners.get(token_id).cloned() {
            Some(account) => account,
            None => env::panic_str("Jeton inconnu"),
        }
    }

    /// Supprime les approbations connues d'un jeton.
    ///
    /// `LookupMap` ne s'itère pas : on ne révoque que les approbations vues
    /// (suffisant pour la démonstration, où un seul jeton circule).
    fn clear_approvals(&mut self, token_id: &str) {
        for candidat in APPROVAL_CANDIDATES {
            if let Ok(compte) = candidat.parse::<AccountId>() {
                self.approvals.remove(&(token_id.to_string(), compte));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    fn id(name: &str) -> AccountId {
        name.parse().expect("compte valide")
    }

    fn contexte(signer: &str) {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("test-nft.test.near"))
            .predecessor_account_id(id(signer))
            .signer_account_id(id(signer))
            .build();
        testing_env!(ctx);
    }

    fn collection() -> TestNft {
        contexte("owner.test.near");
        let mut nft = TestNft::new(id("owner.test.near"));
        nft.mint_to(id("vault.test.near"), "1".to_string());
        nft
    }

    #[test]
    fn emission_reservee_a_emetteur() {
        let mut nft = collection();
        contexte("mallory.test.near");
        let resultat = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nft.mint_to(id("mallory.test.near"), "2".to_string())
        }));
        assert!(resultat.is_err());
    }

    #[test]
    fn approbation_par_un_tiers_est_possible() {
        let mut nft = collection();
        contexte("approver.test.near");
        nft.nft_approve(id("mallory.test.near"), "1".to_string());
        assert!(nft.nft_is_approved(id("mallory.test.near"), "1".to_string()));
    }

    #[test]
    fn approbation_vers_soi_meme_est_ignoree() {
        let mut nft = collection();
        contexte("mallory.test.near");
        nft.nft_approve(id("mallory.test.near"), "1".to_string());
        assert!(!nft.nft_is_approved(id("mallory.test.near"), "1".to_string()));
    }

    #[test]
    fn transfert_reserve_au_detenteur() {
        let mut nft = collection();
        contexte("mallory.test.near");
        let resultat = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            nft.nft_transfer(id("mallory.test.near"), "1".to_string())
        }));
        assert!(resultat.is_err());
    }

    #[test]
    fn metadonnées_exposent_le_standard() {
        let nft = collection();
        let meta = nft.nft_metadata();
        assert!(meta.spec.starts_with("nep171"));
        assert_eq!(meta.symbol, "DRYNFT");
        let _ = near_sdk::serde_json::to_string(&meta).expect("sérialisable");
    }
}
