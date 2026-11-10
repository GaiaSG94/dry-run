//! Comptes de la démonstration et fichiers wasm attendus.

use std::path::{Path, PathBuf};

/// Compte racine du bac à sable (créé à la genèse).
pub const ROOT: &str = "test.near";
/// Propriétaire du coffre : déploie, approuve, gèle, change les plafonds.
pub const OWNER: &str = "owner.test.near";
/// Le coffre (contrat `vault`).
pub const VAULT: &str = "vault.test.near";
/// Bénéficiaire honnête.
pub const BOB: &str = "bob.test.near";
/// L'attaquant.
pub const MALLORY: &str = "mallory.test.near";
/// Détient le NFT que `approver` convoite.
pub const NFT_HOLDER: &str = "nft-holder.test.near";

/// Jeton fongible de démonstration.
pub const TEST_TOKEN: &str = "test-token.test.near";
/// Collection NFT de démonstration.
pub const TEST_NFT: &str = "test-nft.test.near";

/// Un contrat de démonstration : fichier wasm attendu et compte cible.
#[derive(Clone, Copy, Debug)]
pub struct ContractSpec {
    /// Nom du fichier sans extension (underscores, comme le veut la consigne).
    pub wasm: &'static str,
    /// Compte sur lequel le code est déployé.
    pub account: &'static str,
    /// Le contrat est-il déployé puis verrouillé (clés limitées) au démarrage ?
    pub is_vault: bool,
}

/// Table des contrats : ordre de déploiement de la démonstration.
pub const CONTRACTS: &[ContractSpec] = &[
    ContractSpec { wasm: "vault", account: VAULT, is_vault: true },
    ContractSpec { wasm: "good_dex", account: "good-dex.test.near", is_vault: false },
    ContractSpec { wasm: "test_token", account: TEST_TOKEN, is_vault: false },
    ContractSpec { wasm: "test_nft", account: TEST_NFT, is_vault: false },
    ContractSpec { wasm: "trap_drainer", account: "drainer.test.near", is_vault: false },
    ContractSpec { wasm: "trap_skimmer", account: "skimmer.test.near", is_vault: false },
    ContractSpec { wasm: "trap_half_step", account: "half-step.test.near", is_vault: false },
    ContractSpec { wasm: "trap_liar", account: "liar.test.near", is_vault: false },
    ContractSpec { wasm: "trap_approver", account: "approver.test.near", is_vault: false },
    ContractSpec { wasm: "trap_shapeshifter", account: "shapeshifter.test.near", is_vault: false },
];

/// Tous les comptes de la démo, dans l'ordre où ils sont créés à la genèse.
pub const GENESIS_ACCOUNTS: &[&str] = &[
    ROOT,
    OWNER,
    VAULT,
    BOB,
    MALLORY,
    NFT_HOLDER,
    TEST_TOKEN,
    TEST_NFT,
    "good-dex.test.near",
    "drainer.test.near",
    "skimmer.test.near",
    "half-step.test.near",
    "liar.test.near",
    "approver.test.near",
    "shapeshifter.test.near",
];

/// Comptes « froids » répliqués dans le bac à sable de simulation pour lire les deltas.
pub const WATCHED_PLAIN: &[&str] = &[BOB, MALLORY, NFT_HOLDER, OWNER, ROOT];

/// Comptes de frais (jamais des bénéficiaires).
pub const FEE_ACCOUNTS: &[&str] = &["near", ROOT];

/// Cherche le wasm d'un contrat ; accepte les tirets et les underscores.
pub fn wasm_path(dir: &Path, name: &str) -> Option<PathBuf> {
    let dashed = name.replace('_', "-");
    for candidate in [name.to_string(), dashed] {
        let path = dir.join(format!("{candidate}.wasm"));
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_wasm_acceptent_les_deux_orthographes() {
        let dir = std::env::temp_dir().join(format!("dryrun-wasm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dossier temporaire");
        std::fs::write(dir.join("good-dex.wasm"), b"x").expect("écriture");
        let found = wasm_path(&dir, "good_dex");
        assert!(found.is_some(), "le tiret doit être accepté");
        std::fs::remove_dir_all(&dir).ok();
    }
}
