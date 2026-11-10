# `dryrun-sim` — le simulateur

Le pare-feu « Dry Run » rejoue chaque transaction dans un bac à sable NEAR local
**avant** de la signer. Ce crate fournit :

- `chain` — la chaîne locale (`near-sandbox`) : comptes de la démo, déploiement
  des contrats, verrouillage du coffre, appels et lectures ;
- `sim` — le rejeu d'une demande : bac à sable jetable, copie d'état, transaction
  signée **sous le nom du coffre**, capture des effets ;
- `effects` — comparaison avant/après (soldes, jetons, clés, code, reçus) ;
- `rpc` — client JSON-RPC minimal + signature des transactions ;
- `vault_api` — encodage des `VaultAction` et lecture des vues du coffre ;
- `lib` — la façade `Simulator`.

## Façade publique

```rust
pub async fn start(wasm_dir: &Path) -> Result<Simulator>;         // chaîne + déploiement
pub async fn start_undeployed(wasm_dir: &Path) -> Result<Simulator>;
pub async fn deploy_all(&self) -> Result<Deployed>;                // idempotent
pub async fn simulate(&self, &Demande, &Intent) -> Result<Decision>;
pub async fn replay(&self, &Demande, &Intent) -> Result<Replay>;   // effets + issue brute
pub async fn execute(&self, &Demande) -> Result<ExecOutcome>;      // réel, clé d'agent
pub async fn freeze(&self) -> Result<ExecOutcome>;                 // clé de gel (repli propriétaire)
pub async fn approve_contract(&self, account: &str) -> Result<String>;
pub async fn vault_status(&self) -> Result<VaultStatus>;
pub async fn known_contracts(&self) -> Result<BTreeMap<String, String>>;
pub async fn state_of(&self, account: &str) -> Result<AccountState>;
pub async fn view_call(&self, account, method, &Value) -> Result<Value>;
pub async fn height(&self) -> Result<u64>;
pub fn rpc_url(&self) -> &str;
pub fn vault(&self) -> &str;
```

`dryrun_sim::decide(&RuleCtx, &Effects)` est l'unique point de jugement : il
délègue à `dryrun_core::judge`, le simulateur ne juge jamais lui-même.

## Signature : `near-primitives` + `near-crypto`, pas `near-api`

Les transactions sont construites (`TransactionV0`), signées (`SecretKey::sign`)
puis envoyées en JSON-RPC brut (`send_tx`, repli `broadcast_tx_commit`). Raisons :
`near-api` 0.8 embarque sa propre version d'`AccountId` (v2) incompatible avec
celle de `near-sandbox` (v3), et la couche haut niveau masque le nonce et le hash
de bloc dont on a besoin pour rejouer exactement une transaction.

## Chargement d'état : un seul `patch_state`

Un `sandbox_patch_state` coûte une attente de bloc (≈ 0,4 s) **quel que soit le
nombre de documents**. On lit donc l'état sur la chaîne soi-même
(`view_account`, `view_state`, `view_code`, `view_access_key_list`) et on pousse
tous les `StateRecord` en **un seul** patch.

Compromis de volume, pour tenir les budgets de temps :

| Compte | Copié |
|---|---|
| coffre `vault.test.near` | compte + stockage + code + clés (+ clé jetable plein accès) |
| contrat visé par un `FunctionCall` | compte + stockage + code + clés |
| jeton `test-token.test.near` | compte + stockage + code + clés |
| bénéficiaires, `owner`, `mallory`, `nft-holder` | compte + clés seulement |

Les comptes « légers » suffisent : ils ne reçoivent que des fonds, et leurs
soldes sont relus avant/après. Ne copier leur code/stockage doublerait le volume
pour rien.

## Un bac à sable de simulation par demande

Le SPEC décrit « deux bacs à sable ». Ici la chaîne est persistante et le bac à
sable de simulation est **recréé à chaque simulation** (≈ 0,55 s) : aucune trace
d'une simulation hostile (clés ajoutées, stockage créé) ne survit, et la règle 3
(même nom de compte, même état, même solde) est vraie par construction. Les deux
bacs à sable tournent en même temps sans conflit de port.

Mesures locales (contrats déjà déployés) : **≈ 1,6 s** pour une demande
(0,55 s de démarrage + 0,57 s de copie d'état + 0,05 s + ~0,3 s de bloc), bien en
dessous des 3 s visés ; le premier contact ajoute le déploiement des contrats.

## Décisions de représentation

- **Empreinte de code** : le `code_hash` du nœud, c'est-à-dire le SHA-256 des
  octets wasm, en base58. Stable et directement comparable à ce que le coffre
  stocke dans `known`.
- **`stranded`** : vrai seulement si un reçu échoue **et** qu'un tiers (hors
  coffre et hors comptes de frais) a effectivement gagné des fonds. Un simple
  solde négatif du coffre ne suffit pas : les frais de gas en font toujours
  partie.
- **Transaction refusée avant exécution** (`LackBalanceForState`, nonce, …) :
  aucun fonds ne bouge, mais la sortie **demandée** est rapportée dans
  `near_delta` (delta négatif du coffre) avec un reçu en échec. Le pare-feu ne
  doit jamais conclure ALLOW faute d'avoir pu observer quelque chose.
- **Clé jetable du simulateur** (`ed25519:5BGSaf6…`, clé de génèse publique du
  bac à sable) : filtrée des `KeyEvent`, sinon chaque simulation signalerait une
  clé ajoutée sur le coffre.

## Prérequis de la démonstration

Fichiers wasm dans `wasm_dir` (underscores ou tirets, les deux sont acceptés) :

```
vault.wasm              → vault.test.near        (déployé puis verrouillé)
good_dex.wasm           → good-dex.test.near
test_token.wasm         → test-token.test.near
test_nft.wasm           → test-nft.test.near
trap_drainer.wasm       → drainer.test.near
trap_skimmer.wasm       → skimmer.test.near
trap_half_step.wasm     → half-step.test.near
trap_liar.wasm          → liar.test.near
trap_approver.wasm      → approver.test.near
trap_shapeshifter.wasm  → shapeshifter.test.near
```

Un wasm absent est simplement ignoré (les autres agents peuvent compiler en
parallèle) — la variable `DRYRUN_WASM_DIR` change le dossier.

Comptes créés à la genèse : `owner`, `vault` (50 NEAR), `bob`, `mallory`,
`nft-holder` et les comptes des contrats (100 NEAR chacun).
`test.near` existe déjà dans la genèse de nearcore (compte racine, clés de
génèse) : il n'est pas recréé ; c'est un compte de frais.

Après déploiement du coffre, `Deployed::vault_locked` vérifie qu'il ne reste
**aucune** clé `FullAccess` : il ne reste que la clé d'agent (`execute`,
allocation 1 NEAR) et la clé de gel (`freeze`).

## Écart connu avec la SPEC : allocation de la clé de gel

La SPEC demande une clé de gel à **allocation 0**. Sur NEAR, le gas d'une
transaction est prélevé sur l'allocation : une allocation nulle rend la clé
inutilisable (`NotEnoughAllowance`). La clé est donc installée avec 0 comme
demandé, et `Chain::freeze` se replie sur le propriétaire en le signalant dans
les journaux du résultat. Passer `FREEZE_ALLOWANCE` à ~`10^21` yocto rend la clé
de gardien autonome.

## Tests

```
cargo test -p dryrun-sim              # unitaires rapides + intégration réelle
DRYRUN_WASM_DIR=… cargo test -p dryrun-sim
```

- `tests/regles.rs` — 9 tests sans bac à sable : fabrication des `Effects`,
  verdicts, détection NFT/clés/`stranded`, encodage des actions ;
- `tests/bac_a_sable.rs` — 2 tests réels : transfert honnête (ALLOW, vault −1,
  bob +1) puis « 49 NEAR à mallory » (BLOCK, R1) ;
- `tests/vault.rs` — déploiement verrouillé, `status()`, exécution par la clé
  d'agent, refus `ERR_OVER_TX_CAP`, gel puis `ERR_FROZEN`.

Les bacs à sable ne se parallélisent pas : les tests d'un même fichier sont
sérialisés par un verrou, et `cargo test` exécute les fichiers l'un après
l'autre. `tests/vault.rs` s'ignore de lui-même si `vault.wasm` est absent.
