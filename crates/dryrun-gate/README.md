# dryrun-gate — le garde-barrière

Pare-feu qui **simule avant de signer**. Une demande suit toujours le même
chemin : simulation locale → verdict (`ALLOW` / `WARN` / `BLOCK`) → signature par
la clé limitée de l'agent, ou refus sans rien signer.

```bash
cargo run -p dryrun-gate -- propose --to bob.test.near --amount 1 \
      --intent "payer 1 NEAR à bob" --beneficiary bob.test.near
cargo run -p dryrun-gate -- propose --call good-dex.test.near --method swap \
      --deposit 2 --args '{"min_out":1}' --intent "swapper 2 NEAR" --min-in TEST:1.9
cargo run -p dryrun-gate -- approve good-dex.test.near
cargo run -p dryrun-gate -- freeze
cargo run -p dryrun-gate -- status
cargo run -p dryrun-gate -- log
cargo run -p dryrun-gate -- serve --port 8787
cargo run -p dryrun-gate -- demo      # scénarios guidés
cargo run -p dryrun-gate -- attack    # corpus de pièges
```

`--intent` est **obligatoire** pour `propose` : l'intention déclarée est ce qui
borne le pare-feu (bénéficiaires autorisés, minimums attendus).

## Clés : ce que le garde-barrière peut et ne peut pas faire

Le garde-barrière ne détient que la **clé limitée de l'agent**
(`FunctionCall` sur `execute`). Il ne peut ni changer les plafonds, ni approuver
un contrat, ni dégeler. Un `WARN` demande donc une décision du **propriétaire** :
dans la démo locale, la clé du propriétaire vit dans le même processus (raccourci
assumé) ; en production, l'approbation se signe depuis l'appareil du propriétaire,
puis la demande est rejouée.

## Simulateur : `real-sim` (défaut) ou `stub`

| Feature | Simulateur | Usage |
|---|---|---|
| `real-sim` (**défaut**) | `dryrun_sim::Simulator` (SPEC §4), adapté par `src/sim_real.rs` | vraie simulation NEAR |
| `stub` | `src/sim_stub.rs`, coffre en mémoire | compiler et tester le crate seul, sans `near-sandbox` |

```bash
cargo build -p dryrun-gate                                  # vrai simulateur
cargo build -p dryrun-gate --no-default-features --features stub
cargo run   -p dryrun-gate -- --sim stub serve --port 8787   # sans recompiler
```

Le bouchon respecte exactement la signature du SPEC (`start`, `deploy_all`,
`simulate`, `execute`, `vault_status`, `known_contracts`) : rien d'autre ne change.
Si le vrai simulateur ne démarre pas, le garde-barrière **prévient** et se replie
sur le bouchon ; `/api/status` expose alors `"backend": "stub"` et la page affiche
un bandeau d'avertissement.

Limite connue du chemin réel : `dryrun-sim` n'expose pas encore `freeze()` ni
`approve_contract()` depuis sa façade (la clé de gel existe dans sa `Chain`).

## API HTTP

| Route | Méthode | Réponse |
|---|---|---|
| `/` | GET | page statique de `web/` |
| `/api/status` | GET | `VaultStatus` |
| `/api/log` | GET | tableau de `JournalEntryView` |
| `/api/propose` | POST | `Outcome` (`verdict`, `rules`, `card`, `card_view`, `executed`, `note`) |
| `/api/freeze` | POST | `VaultStatus` gelé |

Les montants sont des **chaînes décimales en yoctoNEAR** (aucun flottant).

## Tests

```bash
cargo test -p dryrun-gate
```

`tests/http_api.rs` démarre le serveur sur un port libre et interroge les routes
avec `reqwest` : le simulateur bouchon évite tout bac à sable NEAR.
