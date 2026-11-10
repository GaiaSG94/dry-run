# Spécification interne « Dry Run »

Contrat d'interface entre les morceaux du dépôt. Tout le code respecte ce document.

## 1. Crate `dryrun-core` (déjà écrit, ne pas modifier sans raison)

Types : `Demande`, `Intent`, `Effects`, `Verdict`, `RuleHit`, `Decision`, `ReceiptInfo`, `NearDelta`,
`FtDelta`, `NftEvent`, `KeyEvent`, `CodeEvent`, `Snapshot`, `AccountSnap`.

Règles : `dryrun_core::judge(&RuleCtx, &Effects) -> Decision`, contexte `RuleCtx`
(vault, demand, intent, known, current_code, tx_cap, daily_cap, daily_spent, fee_accounts,
gas_threshold, storage_threshold, max_touched).

Carte : `dryrun_core::card::render(&Decision, &Intent) -> String`.

## 2. Contrat `vault` (`contracts/vault`)

near-sdk 5, nom de crate `vault`.

```rust
pub enum VaultAction {
    Transfer { receiver_id: AccountId, amount: U128 },
    Call { receiver_id: AccountId, method_name: String, args: String, deposit: U128, gas: Gas },
}

#[near(serializers = [json])]
impl Vault {
    pub fn new(owner: AccountId, tx_cap: U128, daily_cap: U128);
    pub fn execute(&mut self, action: VaultAction);            // clé de l'agent
    pub fn approve_contract(&mut self, account_id: AccountId, code_hash: String); // propriétaire, 1 yocto
    pub fn forget_contract(&mut self, account_id: AccountId);  // propriétaire
    pub fn freeze(&mut self);                                 // propriétaire ou clé de gel
    pub fn unfreeze(&mut self);                               // propriétaire, 1 yocto, après délai
    pub fn set_caps(&mut self, tx_cap: U128, daily_cap: U128);// propriétaire, 1 yocto
    pub fn set_sim_key(&mut self, public_key: String);        // propriétaire, 1 yocto (V1)
    // vues
    pub fn status(&self) -> VaultStatus;
    pub fn caps(&self) -> Caps;
    pub fn spent_24h(&self) -> U128;
    pub fn known(&self, account_id: AccountId) -> Option<String>;
    pub fn check(&self, action: VaultAction) -> bool;         // règles on-chain seules, n'exécute rien
    pub fn log(&self, from: u64, limit: u32) -> Vec<LogEntry>;
}
```

Types auxiliaires (sérialisés en JSON) : `VaultStatus`, `Caps`, `LogEntry`.

Événements NEP-297 : `executed`, `refused { reason }`, `frozen`, `unfrozen`, `contract_approved`,
`postcondition_failed`, `hostile_marked`.

Codes de refus : `ERR_FROZEN`, `ERR_OVER_TX_CAP`, `ERR_OVER_DAILY_CAP`, `ERR_UNKNOWN_CONTRACT`,
`ERR_HOSTILE`, `ERR_NO_CERT`, `ERR_CERT_STALE`, `ERR_CERT_MISMATCH`, `ERR_CERT_USED`,
`ERR_BAD_SIGNATURE`, `ERR_COOLDOWN`.

Invariants à tester : I1 (plafond 24 h glissant), I2 (gel), I3 (contrat inconnu refusé),
I4 (seul le propriétaire change plafonds/contrats/gel), I5 (dépense comptée = sorties réussies),
I7 (la clé de gel ne peut que geler).

## 3. Contrats de démonstration

| Crate | Compte | Rôle |
|---|---|---|
| `contracts/good-dex` | `good-dex.test.near` | échange honnête : `swap { token_out, min_out }`, dépose NEAR, rend le jeton |
| `contracts/test-token` | `test-token.test.near` | jeton fongible minimal : `ft_transfer`, `ft_transfer_call`, `ft_on_transfer` |
| `contracts/test-nft` | `test-nft.test.near` | NFT minimal : `nft_transfer`, `nft_approve` |
| `contracts/trap-drainer` | `drainer.test.near` | encaisse le dépôt, ne rend rien (R8) |
| `contracts/trap-skimmer` | `skimmer.test.near` | verse 20 % à un tiers (R7, R8) |
| `contracts/trap-half-step` | `half-step.test.near` | prend les fonds puis échoue au reçu suivant (R9) |
| `contracts/trap-liar` | `liar.test.near` | répond « succès » sans rien transférer (R8) |
| `contracts/trap-approver` | `approver.test.near` | se fait approuver un NFT du coffre (R3) |
| `contracts/trap-shapeshifter` | `shapeshifter.test.near` | honnête, puis son code est remplacé (R6) |

Chaque contrat de piège expose une méthode `trap(args: String)` (ou `swap` pour `good-dex`)
qui est appelée par le coffre via `VaultAction::Call`.

Le jeton `TEST` est minté par `test-token` à `owner.test.near` ; le coffre en reçoit
à la demande du propriétaire (`mint_to` réservé au propriétaire du jeton).

## 4. Crate `dryrun-sim` (simulateur)

```rust
pub struct Simulator { /* chaîne + bac à sable de simulation */ }

impl Simulator {
    /// Démarre la chaîne locale et le bac à sable de simulation.
    pub async fn start(wasm_dir: &Path) -> anyhow::Result<Self>;
    /// Déploie tous les contrats de démonstration sur la chaîne.
    pub async fn deploy_all(&self) -> anyhow::Result<Deployed>;
    /// Simule une demande et rend la décision.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Decision>;
    /// Exécute réellement une demande sur la chaîne (après ALLOW).
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome>;
    /// État du coffre lu sur la chaîne.
    pub async fn vault_status(&self) -> anyhow::Result<VaultStatus>;
    /// Contrats connus lus sur la chaîne.
    pub async fn known_contracts(&self) -> anyhow::Result<BTreeMap<String, String>>;
}
```

Mécanique : deux bacs à sable `near-sandbox`.
- la **chaîne** porte les contrats et les comptes ;
- le **simulateur** recharge l'état de la chaîne par `patch_state().fetch_from(rpc, FetchData::ALL)`
  pour le coffre et le contrat visé, y pose une clé jetable plein accès sur le coffre
  (`with_default_access_key`), joue la transaction **en tant que le coffre**, attend les reçus,
  puis compare les soldes, les clés, le code et les reçus avant/après.

Règle 3 du cahier des charges : la simulation se fait avec le même nom de compte que le coffre.

## 5. Crate `dryrun-gate` (garde-barrière)

```rust
pub struct Gatekeeper { /* simulator + clés */ }

impl Gatekeeper {
    pub async fn start(wasm_dir: &Path) -> anyhow::Result<Self>;
    /// Une demande passe par : simulation → verdict → signature ou refus.
    pub async fn propose(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Outcome>;
    pub async fn freeze(&self) -> anyhow::Result<()>;
    pub async fn status(&self) -> anyhow::Result<VaultStatus>;
}

pub struct Outcome { pub verdict: Verdict, pub rules: Vec<RuleHit>, pub card: String, pub executed: bool }
```

Ligne de commande :

```
dryrun-gate propose --to bob.test.near --amount 1 --intent "payer 1 NEAR à bob"
dryrun-gate propose --call good-dex.test.near --method swap --deposit 2 --intent "..."
dryrun-gate freeze
dryrun-gate status
dryrun-gate serve --port 8787      # page web + API JSON
```

Le garde-barrière ne détient **que** la clé limitée de l'agent : il ne peut pas
changer les plafonds, ni approuver un contrat, ni dégeler.

## 6. Règles de code (impératives)

- un fichier < 500 lignes, une fonction < 50 lignes ;
- zéro duplication :factoriser dans `dryrun-core` ou un module partagé ;
- pas de `unwrap()` dans le code de production, sauf dans les tests ;
- commentaires en français, identifiants en anglais ;
- pas de `node -e` : tout script passe par un fichier ;
- la compilation se fait avec `cargo` ; le wasm des contrats avec
  `cargo near build non-reproducible` ou `cargo build --target wasm32-unknown-unknown --release`.

## 7. Ce qui est interdit

- pas de secret dans le dépôt ;
- pas de dépendance réseau au moment de la démo locale (tout est local) ;
- pas de `rm -rf` : utiliser `trash` quand c'est possible ;
- ne pas modifier le contrat d'interface ci-dessus sans mettre à jour ce document.

## 3.1 Comptes de la démo

| Compte | Rôle |
|---|---|
| `test.near` | compte racine du bac à sable (détient les clés de génèse) |
| `owner.test.near` | propriétaire : déploie, approuve les contrats, gèle, change les plafonds |
| `vault.test.near` | le coffre (contrat `vault`) |
| `bob.test.near` | bénéficiaire honnête |
| `mallory.test.near` | l'attaquant (destinataire de « envoie tout à X ») |
| `nft-holder.test.near` | détient le NFT que `approver` va vouloir faire approuver |

Le coffre est Approvée par le propriétaire avec une clé d'agent **limitée à l'appel de fonction**
`execute`, et une clé de gel **limitée à l'appel de fonction** `freeze`.
Après l'installation, le compte `vault.test.near` n'a **plus aucune clé d'accès total** :
la démonstration vérifie que le compte du coffre n'a pas de clé `FullAccess`.
