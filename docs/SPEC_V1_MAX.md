# Spécification V1 & MAX « Dry Run »

Ce document complète `SPEC.md` avec les spécifications techniques exactes pour la V1 et MAX.

## 1. Certificat de simulation Ed25519 vérifié On-Chain (V1)

### 1.1 Structure du certificat (`dryrun-core::types::Cert`)

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cert {
    /// SHA-256 de la demande (digest déterministe).
    pub call_digest: [u8; 32],
    /// Hauteur du snapshot simulé.
    pub snapshot_height: u64,
    /// Hauteur d'expiration (snapshot_height + 20 blocs).
    pub expires_height: u64,
    /// Montant maximum de sortie autorisé en yoctoNEAR.
    pub max_out: u128,
    /// Post-conditions : minimum attendu en retour `(token_account, min_amount)`.
    pub post: Vec<(String, u128)>,
    /// Nonce aléatoire ou séquentiel unique (anti-rejeu).
    pub nonce: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedCert {
    pub cert: Cert,
    /// Signature Ed25519 (64 octets), encodée en base58 ou hex.
    pub signature: String,
}
```

### 1.2 Vérification on-chain dans `vault::execute`

Dans `contracts/vault/src/lib.rs` :
- Méthode `execute` accepte désormais `action: VaultAction, cert: Option<SignedCert>` (ou `execute_certified(action, cert)` pour rétrocompatibilité).
- Si `sim_key` est configuré sur le vault :
  1. `cert` est obligatoire (`ERR_NO_CERT`).
  2. `cert.expires_height >= env::block_height()` (`ERR_CERT_STALE`).
  3. `!self.used_nonces.contains(&cert.nonce)` (`ERR_CERT_USED`). Enregistre le nonce.
  4. `cert.call_digest == sha256(action)` (`ERR_CERT_MISMATCH`).
  5. `cert.max_out >= action.amount()` (`ERR_OVER_TX_CAP`).
  6. Vérification cryptographique de la signature via `env::ed25519_verify(&sig, &cert_bytes, &self.sim_key)` (`ERR_BAD_SIGNATURE`).
- Si tout est valide, lance la transaction et attache `on_done` avec les post-conditions (`cert.post`).

## 2. Contrôle au retour & Marquage hostile (`on_done`)

Dans `contracts/vault/src/lib.rs` :
- `on_done` vérifie :
  1. Si le reçu distant a échoué -> annule la dépense comptée (déjà fait, I5).
  2. Si des post-conditions (`post`) sont spécifiées :
     - Vérifie que le solde du jeton ou NEAR a augmenté d'au moins `min_amount`.
     - Si écart -> déclenche `self.freeze()`, ajoute le contrat appelé à `self.hostile: BTreeSet<AccountId>`, et émet `events::hostile_marked(&target)`.

## 3. Simulation différentielle (V1)

Dans `crates/dryrun-sim` :
- `simulate_differential(demand, intent) -> Result<Decision>` :
  - Joue la transaction au montant réel $M$.
  - Joue la même transaction avec $M / 100$ (montant réduit d'un centième).
  - Compare le ratio obtenu : si le taux reçu par unité dépensée diverge de plus de 5% (sans courbe de bonding / slippage déclaré dans l'intention) -> `BLOCK (R12: contrat à double visage / sandbox evasion)`.

## 4. Benchmarking par robots (`make sim`)

Crate `crates/dryrun-bench` ou sous-commande `dryrun-gate sim` :
- Génère 200 demandes aléatoires :
  - 100 demandes honnêtes (transferts normaux, swaps good-dex avec slippage raisonnable).
  - 100 attaques (drainers, skimmers, injections, dépassement de plafonds, half-steps, faux jetons).
- Exécute et calcule la matrice de confusion :
  - True Positives (attaques bloquées)
  - True Negatives (honnêtes acceptés)
  - False Positives (honnêtes bloqués à tort)
  - False Negatives (attaques manquées)
  - Latence médiane, p95, p99.
- Affiche un tableau clair et sauvegarde `bench_matrix.json`.

## 5. Serveur MCP (Model Context Protocol)

Dans `crates/dryrun-mcp` (binaire `dryrun-mcp`) :
- Protocole JSON-RPC standard sur stdin/stdout.
- Outils :
  - `wallet_status` : solde, plafonds 24h, contrats connus, état gelé.
  - `wallet_propose` : reçoit `demand` + `intent`, fait la simulation, renvoie la décision (`ALLOW`/`WARN`/`BLOCK`) et la carte lisible.
  - `wallet_freeze` : déclenche le gel d'urgence.
  - `wallet_explain` : explique en langage naturel pourquoi une règle a mordu.

## 6. Finitions UI Web

Dans `web/` :
- Panneau de scénarios rapides (1 clic) :
  1. Transfert honnête (1 NEAR à bob)
  2. Injection "tout vider" (49 NEAR à mallory)
  3. Contrat honnête (good-dex swap)
  4. Piège Drainer (encaisse sans rien rendre)
  5. Piège Skimmer (rétro-commission cachée 20%)
  6. Piège Approver (tentative d'approbation NFT)
- Panneau repliable "Reçus & Logs NEAR détaillés" sous la carte des effets.
- Indicateur de statut de connexion live avec auto-refresh toutes les 3s.
