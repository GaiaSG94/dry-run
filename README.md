# Dry Run — a simulation firewall for NEAR agent wallets

> **"Every transaction happens twice, the first time for nothing."**
>
> NEAR HACKATHON project — track 06 `security` ("AI Security & Defense").
> Built 10–14 November 2026 (first commit on 10/11, submission on 14/11 at 23:59 UTC).
> Built by **GaiaSG94** (@GaiaSG94).
> Repository: <https://github.com/GaiaSG94/dry-run>
> Live demo (scripted, no install): <https://gaiasg94.github.io/dry-run/?lang=en>
> Other NEAR HACKATHON project by the same builder (independent): [Hireclaw](https://github.com/GaiaSG94/hireclaw), track Agent Economy.

---

## 1. Why this project?

Firewalls for AI agents judge **the request**: they look at the announced recipient and amount and compare them with allow-lists and caps. What the called contract will really do with the funds is never examined.

On NEAR, that model is especially dangerous:
- **NEAR has no native RPC simulation**: `view` calls are read-only and do not follow cross-contract calls.
- **A cross-contract call cannot be rolled back**: if the second receipt fails, the first one has already taken the funds.
- **"Unlimited approval" exists under other names**: adding a full-access key, an NFT approval (`nft_approve`), or a transfer-and-call (`ft_transfer_call`) to an unknown contract.

**Dry Run solves this with a real local execution before any signature:**
1. The transaction is played in a **local sandbox** (`near-sandbox`) synchronised with the chain state;
2. The simulation runs **as the vault** (same account name, same balances, same state);
3. The verdict comes from the **real effects** (balances before/after, added keys, approvals, failed receipts, contracts touched), not from the agent's text;
4. The vault enforces rolling caps and the freeze **on-chain**, whatever happens to the simulator.

---

## 1 bis. Why caps alone are not enough

The fair objection a reviewer makes: **the vault already refuses a 49 NEAR transfer with
`ERR_OVER_TX_CAP` — isn't a spending cap enough?**

Caps only look at **what the request says**. They cannot see **what the contract will really do**
with the funds, and on NEAR a cross-contract call cannot be rolled back. A cap answers "how much
can leave", never "where does it go and what comes back".

| What a cap sees (R1, on-chain) | What a cap cannot see |
|---|---|
| an amount above the transaction or 24 h cap | a beneficiary never named in the intent (R7) |
| a plain transfer | a `ft_transfer_call` to an unknown contract (R4) |
| the amount announced | the amount actually returned: "promised 1.9 TEST, received 1.56" (R8) |
| a single call | funds taken, then a failed receipt that cannot be undone (R9) |
| the account it is sent to | an NFT approval granted on the way (R3) |
| the contract named today | the same contract with **replaced code** after the approval (R6) |
| the first call to a never-seen contract | nothing: the cap has no notion of "first contact" (R5, WARN) |
| nothing about keys | an access key added on the vault account (R2) |
| nothing about cost | abnormal gas or storage deposit (R10, WARN) |
| nothing about shape | a call touching more than 8 accounts (R11) |

The vault's on-chain caps (2 NEAR per transaction, 10 NEAR per 24 h) are the **floor**, not the
ceiling of the defence: R1 is one of the twelve rules, and the other eleven judge the effects of
the call rather than its wording.

**And the key is not the only defence.** On-chain, the vault executes nothing that is not
accompanied by a valid V1 simulation certificate (section 5 bis), so a stolen agent key is no
longer enough by itself.

---

## 2. One-command demo (Do X → See Y)

No secret, no paid token, no external service: the demo runs entirely locally.

```bash
# 1. Clone
git clone https://github.com/GaiaSG94/dry-run.git
cd dry-run

# 2. Build the contracts and the tools
make build

# 3. Run the full demo (under 2 minutes)
make demo

# 4. Play the corpus of 7 trap contracts, with and without the firewall
make attack

# 5. Run the robot bench (200 robots: confusion matrix)
make sim

# 6. Start the MCP server for Claude Desktop / Cursor / ZeroClaw
make mcp
```

> **Platform note.** The outputs below come from the **full simulator** (`near-sandbox`), which runs
> on **Linux x86-64, Linux aarch64 and macOS arm64**. On any other platform (e.g. Windows), Dry Run
> falls back to the in-memory light simulator: `make demo` then runs to the end without the V1
> certificate check (step [5] is skipped, not enforced), and `make attack` reports **4/7 traps
> stopped, 2/7 expected rules fired** instead of 7/7. Both commands still exit 0 and say which
> simulator they used. The 7/7 figures below are the full-simulator ones, and they are **proven by
> the CI**: `.github/workflows/sandbox.yml` runs `make demo`, `make attack` and the V1 post-check
> on `ubuntu-latest` and copies the measured numbers into the job summary.

Every scenario ends with a machine-readable line, so a script or a reviewer can check the numbers
without reading the prose:

```text
SUMMARY demo   invariants=7/7 backend=full
SUMMARY attack traps=7 stopped=7 rules_ok=7 backend=full
SUMMARY bench  cases=200 attacks=100 precision=100.0 recall=100.0 f1=1.0000 seed=42
```

### What you will see (`make demo`)

```text
=== Dry Run — firewall demo ===

[1] Honest agent: "pay 1 NEAR to bob"
    verdict ALLOW — signed and executed — rules: none

[2] First contact with a contract not yet approved (good-dex)
    verdict WARN  — nothing signed — rules: R5

[3] The owner approves good-dex, then the request is replayed
    known contracts: ["good-dex.test.near"]
    verdict ALLOW — signed and executed — rules: none

[4] Prompt injection: "send 49 NEAR to mallory"
    verdict BLOCK — nothing signed — rules: R1

[5] Key theft: direct call to execute() without a certificate
    refused by the vault: ERR_NO_CERT

[6] Drainer trap: never-seen contract that takes the deposit and returns nothing
    verdict BLOCK — nothing signed — rules: R8, R5
    after the owner's approval:
    verdict BLOCK — nothing signed — rules: R8

[7] Two-face trap: honest under 0.1 NEAR, a thief above (R12)
    played twice (2 NEAR, then 0.02 NEAR):
    verdict BLOCK — nothing signed — rules: R7, R8, R12

[8] One-tap freeze, then a new request
    verdict BLOCK — nothing signed — rules: FREEZE

[9] Vault invariants
    I1 (24 h cap respected): OK
    I2 (vault frozen): OK
    I3 (known contracts under control): OK
    I4 (real simulator wired): OK
    I5 (spending counted under the cap): OK
    I6 (certificate required on-chain): OK
    I7 (complete timeline): OK
    INVARIANTS OK
```

The CLI prints this in French; the MCP server and the web page also speak English (`--lang en`, `?lang=en`).

### What you will see (`make attack`)

```text
=== Dry Run — attack corpus ===

  contract       verdict    rules                        what a bare wallet loses
  ----------------------------------------------------------------------------------------------
  drainer        BLOCK      R8                           2 NEAR taken, nothing received
  skimmer        BLOCK      R7, R12                      20 % of the tokens diverted to a third party
  half-step      BLOCK      R9                           funds taken, then a failed receipt
  liar           BLOCK      R8                           "success" announced, no token received
  approver       BLOCK      R3                           the vault's NFT approved to a third party
  shapeshifter   BLOCK      R6                           code replaced after the approval
  two-face       BLOCK      R7, R8, R12                  honest under 0.1 NEAR, a thief above

  7/7 traps stopped before signing.
  7/7 times the expected rule fired.
  (full simulator — Linux/macOS; on the light simulator see the platform note above)
```

Each trap is played **after** the owner has approved the contract: the worst case for the
firewall. The rule that fires is the one that describes the observed effect, not the allow-list.

---

## 3. Architecture

```text
                +-------------------------------------------------------+
                |                       AI agent                        |
                +-------------------------------------------------------+
                                           | propose(action, intent)
                                           v
+------------------------------------------------------------------------------------+
|                                     Gatekeeper                                     |
|   (holds ONLY the FunctionCall::execute key, can neither change nor unfreeze)      |
+------------------------------------------------------------------------------------+
       |                                                         |
       | 1. simulate                                             | 3. if ALLOW: sign
       v                                                         v
+-----------------------------+                           +--------------------------+
|       Local simulator       |                           |        NEAR chain        |
| (near-sandbox state clone)  |                           |                          |
|                             |                           |   Vault contract         |
|  - plays as the vault       |                           |   - on-chain caps        |
|  - extracts Effects         |                           |   - known registry       |
|  - applies R1 to R12        |                           |   - watertight freeze    |
|  - returns a Decision       |                           |   - on_done callback     |
+-----------------------------+                           +--------------------------+
       |                                                                 ^
       +---- 2. if BLOCK: refuse, show the effects card                  |
       +---- 2. if WARN:  ask the owner (approve_contract) --------------+
```

### The effects card (what the owner and the judges see)

```text
 PAY 2 NEAR for ≥ 1.9 TEST on good-dex.test.near                  ALLOW
 balances
   vault.test.near                          -2.000000 NEAR
   good-dex.test.near                       +2.000000 NEAR
   vault.test.near                          +1.960000 TEST
 keys, approvals
   none
 receipts
   3, 0 failed
 rules
   none
 simulated at block 42 — gas 12 Tgas — no failed receipt
```

Facing a trap contract (`skimmer`):

```text
 PAY 2 NEAR for ≥ 1.9 TEST on skimmer.test.near                   BLOCK
 balances
   vault.test.near                          -2.000000 NEAR
   vault.test.near                          +1.560000 TEST
   mallory.test.near                        +0.400000 TEST
 keys, approvals
   none
 receipts
   4, 0 failed
 rules
   R7 undeclared beneficiary  — mallory.test.near received 0.4 TEST without being named
   R8 less received than promised — received 1.56 TEST while the intent required ≥ 1.9
 simulated at block 43 — gas 18 Tgas — no failed receipt
```

---

## 4. The firewall's 12 rules

| # | Rule | Verdict | What it stops |
|---|---|---|---|
| **R1** | Output > per-transaction cap or > remaining 24 h cap | `BLOCK` | "Empty the wallet" injections, cumulative drift |
| **R2** | Access key added on the vault account | `BLOCK` | Stealth takeover |
| **R3** | NFT approval (`nft_approve`) to an undeclared third party | `BLOCK` | NFT theft by proxy |
| **R4** | `ft_transfer_call` to an unknown contract | `BLOCK` | Transfer with uncontrolled execution |
| **R5** | First contact with a contract | `WARN` | Interaction not approved by the owner |
| **R6** | Code hash changed since the approval | `BLOCK` | Contract transformed afterwards (`shapeshifter`) |
| **R7** | Beneficiary not declared in the intent | `BLOCK` | Hidden kickbacks (`skimmer`) |
| **R8** | Less received than the promised minimum, or nothing | `BLOCK` | Drainers, lies (`liar`, `drainer`) |
| **R9** | Failed receipt after funds left (`stranded`) | `BLOCK` | Half-way contracts (`half-step`) |
| **R10** | Abnormally high gas or storage deposit | `WARN` | Gas bombs, storage hogs |
| **R11** | More than 8 accounts touched | `BLOCK` | Transactions too complex to be certified |
| **R12** | Differential divergence, real vs reduced amount (V1) | `BLOCK` | Two-faced contracts, sandbox evasion |

---

## 5. The corpus of trap contracts (`make attack`)

Each trap contract is played **once through Dry Run**; the **control** (a bare wallet without a
firewall) is played **after** the whole corpus, on the protected vault, to show what the same call
does without the gate (see section 9 bis).

| Contract | What it does | Without firewall (bare) | Through Dry Run | Rule |
|---|---|---|---|---|
| `drainer` | Takes the deposit, returns nothing | −2 NEAR lost | **Refused** before signing | `R8` |
| `skimmer` | Diverts 20 % of the tokens to a third party | 0.4 TEST stolen | **Refused** before signing | `R7` |
| `half-step` | Takes the funds, then fails the next receipt | Funds stuck | **Refused** before signing | `R9` |
| `liar` | Answers "success" without transferring anything | −2 NEAR for nothing | **Refused** before signing | `R8` |
| `approver` | Tries to get an NFT approved on the way | NFT exposed | **Refused** before signing | `R3` |
| `shapeshifter` | Code replaced after the approval | Contract altered | **Refused** before signing | `R6` |
| `two-face` | Honest under 0.1 NEAR, a thief above | 95 % of the tokens diverted | **Refused** before signing | `R12` |
| Key theft (V1 level) | Direct call to the vault with the stolen agent key | N/A | **Rejected on-chain**: `ERR_NO_CERT` | On-chain |

**Measured result**: 7/7 traps stopped before signing, 7/7 times the expected rule fired
(full simulator — Linux/macOS; see the platform note in section 2).

> **V1 note — post-check enabled.** The promise chain
> `call → on_done → balance read → on_post_check` now fits in the gas budget of an agent
> call (300 TGas attached, 100 TGas for the target call, 10 TGas per callback): if the balance
> promised by the certificate is not reached on return, the vault **freezes** and marks the
> contract **hostile** (`ERR_HOSTILE` for any later call). Proven by the integration test
> `crates/dryrun-sim/tests/v1_post_check.rs` (real sandbox).

---

## 5 bis. Level V1: the simulation certificate, verified on-chain

This is what sets Dry Run apart:

> The vault **executes nothing** that is not accompanied by a cryptographic certificate
> signed by the simulator's key.

```text
Cert {
  call_digest  : SHA-256 of the canonical form of the request
  snapshot_height, expires_height : freshness (20 blocks)
  max_out      : maximum output allowed
  post         : (token, minimum) promised in return
  nonce        : single use
}
signature = Ed25519(simulator key, Cert)
```

Checks made **by the contract, on-chain**, before any output:

| Check | Refusal code |
|---|---|
| Certificate present | `ERR_NO_CERT` |
| Nonce never used | `ERR_CERT_USED` |
| Certificate still fresh | `ERR_CERT_STALE` |
| The digest matches the received request | `ERR_CERT_MISMATCH` |
| The output does not exceed the certified cap | `ERR_OVER_TX_CAP` |
| Valid Ed25519 signature for the registered key | `ERR_BAD_SIGNATURE` |
| The called contract is not marked hostile | `ERR_HOSTILE` |

**Direct consequence**: stealing the agent's key is no longer enough. The attacker would also
need the simulator's key, which signs **no** transaction and therefore cannot spend anything alone.

```text
$ cargo run --release -p dryrun-gate -- demo
[5] Key theft: direct call to execute() without a certificate
    refused by the vault: ERR_NO_CERT
```

The simulator's public key is registered on the vault by the owner (`set_sim_key`,
1 yoctoNEAR), like any administrative action on the vault.

---

## 5 ter. The two defences that follow the transaction

### The post-check (`on_post_check`)

The certificate does not only authorise the output: it **promises a result**.
`post = (token, minimum balance expected after the call)` is signed by the simulator, then
checked **on-chain, after** the execution:

```text
target call → on_done (refunds the spend if the receipt failed)
            → ft_balance_of(token)        reads the real balance
            → on_post_check               balance < promised minimum?
                                            ├─ the vault freezes immediately
                                            └─ the contract is marked HOSTILE (NEP-297 event)
```

This answers **sleeper** contracts: a contract that behaves in simulation and steals at
execution is neutralised on return, and can never be called again (`ERR_HOSTILE`). The whole
chain fits in the budget of an agent call: 300 TGas attached, 100 TGas for the target call,
10 TGas per callback.

```text
$ cargo test -p dryrun-sim --test v1_post_check
test le_contrôle_au_retour_gele_et_marque_hostile ... ok
```

> This test needs the real `near-sandbox`, so it runs on **Linux/macOS only** (it fails on
> Windows: "Unsupported platform"). The CI runs the unit tests only
> (`cargo test --workspace --lib --bins`); the integration tests are run locally on Linux/macOS.

### The differential simulation (R12)

Every contract call with a deposit is played **twice**: at the real amount, then at one
hundredth. Each replay starts from a throwaway sandbox and both run **in parallel**
(`tokio::join!`): latency does not double.

The test is **directional**: R12 fires when the small amount is proportionally **better**
treated than the real amount — the signature of a contract that behaves while it is watched.
Received quantities are compared with each other (never divided by the deposit in yoctoNEAR):
the result depends neither on the token's decimals nor on gas fees.

```text
$ cargo run --release -p dryrun-gate -- demo
[7] Two-face trap: honest under 0.1 NEAR, a thief above (R12)
    played twice (2 NEAR, then 0.02 NEAR):
    verdict BLOCK — nothing signed — rules: R7, R8, R12
```

`two-face` returns 95 % of the converted amount on 0.02 NEAR and only 5 % on 2 NEAR:
a measured gap of 947 ‰, far above the 100 ‰ threshold.

---

## 6. The precise role of NEAR Protocol

Dry Run relies on three fundamental NEAR primitives:
1. **Function-call access keys (`FunctionCallAccessKey`)**: they give the agent and the simulator the right to call `execute`, and the guardian the right to call `freeze`, while every `FullAccess` key is removed from the vault account after setup.
2. **The official sandbox (`near-sandbox`)**: it synchronises the real chain state through `patch_state` and executes the transaction locally under the vault's real identity (`with_default_access_key`).
3. **The asynchronous receipt model (`Promise`)**: it is why simulation must come first — cross-contract calls cannot be rolled back transactionally on NEAR.

---

## 6 bis. The vault deployed on NEAR testnet

The `vault` contract runs on the public testnet, on the account
**[`dryrun-vault-1112.testnet`](https://testnet.nearblocks.io/address/dryrun-vault-1112.testnet)**
(deployed on 2026-11-12, caps of 2 NEAR per transaction and 10 NEAR per 24 h).

| Step | Result on-chain | Transaction |
|---|---|---|
| Code deployment | contract installed | [`D6QiwMAK…`](https://explorer.testnet.near.org/transactions/D6QiwMAKBLsvyCEN7hb8Em4fJ1qnZLV9p4nsjHKA7qjV) |
| `new` (initialisation) | owner, `tx_cap` 2 NEAR, `daily_cap` 10 NEAR | [`7JDQgzM8…`](https://explorer.testnet.near.org/transactions/7JDQgzM8TKYCYgL4awiZzKtKEdZeYb5UBk9SFHEMu9Ed) |
| `execute`: 5 NEAR to `testnet` | **refused**: `ERR_OVER_TX_CAP`, nothing leaves | [`3YDpi4y9…`](https://explorer.testnet.near.org/transactions/3YDpi4y9tTBJDBmu5VFCn1zFRPc1K3hBz3MHqQrqnjmB) |
| `execute`: 0.1 NEAR to `testnet` | **executed**, `spent_24h` = 0.1 NEAR | [`HdZr1wuS…`](https://explorer.testnet.near.org/transactions/HdZr1wuSSQSJQ3jcsHhqjzuEgP1opAfv6tdMdFTupck6) |

To read the state:

```bash
near contract call-function as-read-only dryrun-vault-1112.testnet status json-args '{}' network-config testnet now
```

### 6 ter. The firewall wired to the testnet vault

`dryrun-gate` has a **testnet mode**: it no longer judges against an in-memory vault, but
against the `vault` actually deployed above.

```bash
# The MCP server, wired to the chain (the agent really signs)
# Runs as-is: no environment variable needed
dryrun-gate mcp --network testnet --vault dryrun-vault-1112.testnet --lang en

# The web page and its API, wired to the chain
# Also runs as-is, same default endpoint
dryrun-gate serve --network testnet --vault dryrun-vault-1112.testnet
```

The default RPC endpoint is `https://rpc.testnet.fastnear.com` — a public endpoint that works
out of the box. (The historical `rpc.testnet.near.org` is deprecated and rate-limited: it
answers with 429 errors.) `DRYRUN_RPC_URL` remains available as an **optional** override, for
any other provider — accepted only if it starts with `https://`; an `http://` URL is refused
and the default endpoint is used:

```bash
# Optional override of the RPC endpoint (https only)
DRYRUN_RPC_URL=https://archival-rpc.testnet.fastnear.com \
  dryrun-gate serve --network testnet --vault dryrun-vault-1112.testnet
```

What the mode changes, and nothing else:

| | Local mode (default) | `--network testnet` mode |
|---|---|---|
| Vault state | in memory or local sandbox | `status()` read **over RPC** (`https://rpc.testnet.fastnear.com` by default, or `DRYRUN_RPC_URL` if it is an `https` URL) |
| Verdict | same rules R1 to R12 | same rules R1 to R12, **judged against the chain's real caps** |
| `ALLOW` verdict | signed and executed locally | `execute(action, cert: null)` sent **on-chain**, signed by the account key |
| `BLOCK` / `WARN` verdict | nothing is signed | nothing is sent |
| Response | "executed on the local vault (simulation)" | "on NEAR testnet · tx `<hash>`" |

The caps read on-chain are those of the deployed vault: 2 NEAR per transaction, 10 NEAR per
24 h. A 49 NEAR request is therefore blocked by **R1** before any signature, exactly as locally.

**Dry Run never reads the key.** It stays in the system keychain; Dry Run calls the NEAR CLI
(`near-cli-rs`), which fetches it itself (`sign-with-keychain`). The binary is looked up in the
`PATH`, or pointed to by the `NEAR_CLI_BIN` environment variable.

### 6 quater. Verified for real, on-chain

A real JSON-RPC session against the testnet vault, on 2026-11-13:

| Request (MCP client) | Verdict | What happened |
|---|---|---|
| 49 NEAR to `mallory.test.near` | **BLOCK** (R1) | nothing signed, no transaction |
| 0.1 NEAR to `testnet` | **ALLOW** | transaction signed and executed: [`BJqLxihw…`](https://explorer.testnet.near.org/transactions/BJqLxihwSfCpucK8VCHJPXKA7FmHZ2jUvoaivZUryVHa) |

After the transaction, `status()` on-chain confirms the spend: `spent_24h` went from 0.1 to
0.2 NEAR. The transaction itself carries the `execute` call with `cert: null` — the V1
certificate is not required, since this deployment has no registered `sim_key`.

**What this proves:** the firewall protects a **really deployed** wallet. The verdict comes
from a state read on-chain, the signature is the account key's, and the output carries a
clickable transaction hash.

**What this does not prove:**
- The effects of a **contract call** are not replayed off-chain: a call to an unknown
  contract is treated as a first contact (R5, `WARN`) and is therefore not sent without the
  owner's confirmation. A transfer, however, is deterministic: it is judged exactly.
- The emergency freeze is **refused** on the testnet vault (`DRYRUN_ALLOW_TESTNET_FREEZE=1`
  forces it): a real freeze lasts at least one hour and would break the demo for the rest of
  the day.
- For the demo, the vault is its own `owner` and keeps its `FullAccess` key. In production,
  that key is removed and the agent gets a key limited to `execute` (section 6, point 1).
- No `sim_key` is registered: the V1 certificate is not required on this deployment.

---

## 7. Repository layout

```text
dry-run/
├── Cargo.toml                       # Rust workspace
├── Makefile                         # make build, make demo, make attack, make sim, make mcp
├── crates/
│   ├── dryrun-core/                 # Shared types, rule engine (R1-R12), certificate, effects card
│   ├── dryrun-sim/                  # Simulator (near-sandbox, effect capture, Ed25519 certificate, differential simulation)
│   └── dryrun-gate/                 # Gatekeeper (CLI, HTTP server, MCP, testnet mode, 200-robot bench)
├── contracts/
│   ├── vault/                       # The vault (rolling caps, known, freeze, V1 certificate, hostile marking)
│   ├── good-dex/                    # Honest reference DEX
│   ├── test-token/                  # Minimal NEP-141 token (TEST)
│   ├── test-nft/                    # Minimal NEP-171 NFT
│   ├── trap-drainer/                # Trap 1: takes without giving back
│   ├── trap-skimmer/                # Trap 2: kickback to a third party
│   ├── trap-half-step/              # Trap 3: fails after taking the funds
│   ├── trap-liar/                   # Trap 4: promises without transferring
│   ├── trap-approver/               # Trap 5: NFT approval attempt
│   ├── trap-shapeshifter/           # Trap 6: code alteration
│   └── trap-two-face/               # Trap 7: honest under a threshold, a thief above (R12)
├── web/                             # Local web interface (3D scene, live terminal, request buttons, freeze)
├── scripts/                         # Testnet deployment and video recording scripts
└── docs/
    ├── SPEC.md                      # Interface specification (level M)
    └── SPEC_V1_MAX.md               # V1 specification (certificate, hostile marking, differential simulation) and MAX
```

---

## 8. Web interface & HTTP server

The web server runs locally on port 8787 and serves the page and its JSON API.

```bash
cargo run --release -p dryrun-gate -- serve --port 8787
# or simply:
./run-server.sh
```

Then open <http://localhost:8787/?lang=en>. Without a server, the same page runs as a scripted
demo — that is the live demo on GitHub Pages: <https://gaiasg94.github.io/dry-run/?lang=en>.

### 8.1 The 3D scene

One page, one scene: the whole path of a request on a 3D board — **① AI agent → ② firewall →
③ sandbox (the copy of the chain) → ④ the 12 rules → ⑤ verdict → ⑥ vault → ⑦ NEAR chain**.

- **at the top, one white label** says in one sentence what Dry Run does, then narrates each
  step as it happens; just below it, a line says **before anything is played** whether you are
  watching the scripted demo or talking to the real firewall (live mode, with the vault and
  its balance);
- **the requests are buttons under the label**: ▶ play the process, pay invoice · 1 Ⓝ,
  swap good-dex, injection 49 Ⓝ, drainer trap, two-face trap, freeze the vault;
- **a live terminal on the right** logs every step, every effect, every verdict and, in live
  mode, the real network calls and the transaction hash.

Two modes, detected automatically on load:

- **LIVE** (server running): each button sends a real request to the firewall, and the scene
  plays the steps with its real answer (verdict, rules, effects, execution, hash);
- **DEMO** (page opened without a server, e.g. GitHub Pages): the scripted film of the
  real `make demo` results.

Do X → see Y (live mode, `dryrun-gate serve`):

1. **pay invoice · 1 Ⓝ** → the copy shows −1 NEAR, no rule fires → **ALLOW** → executed;
2. **injection 49 Ⓝ** → −49 NEAR in the copy → **BLOCK (R1)**: nothing is signed, the
   explanation goes back to the agent over MCP;
3. **drainer trap** → 2 NEAR go out, nothing comes back → **BLOCK (R8)**: the **effects** stop
   it, not an allow-list;
4. **two-face trap** → **BLOCK**: with the full simulator (`make demo`, `make attack`) the
   **differential simulation (R12)** unmasks it; with the light simulator (`--sim stub`, the
   fallback when `near-sandbox` is unavailable), it is stopped by **R8** (less received than promised);
5. **swap good-dex** → with the full simulator: **WARN (R5, first contact)** → the owner
   confirms (`confirm:true`) → re-simulation → **ALLOW** → the Ed25519 seal closes **7/7** →
   the transaction runs → **"sand = real"**. The light simulator returns no token delta, so
   this honest swap ends **BLOCK (R8)** there — the scripted film shows the full-simulator run;
6. **freeze the vault** → `POST /api/freeze` → **FROZEN**, every later request is refused
   (`ERR_FROZEN`);
7. **▶ play the process** → the full scripted film: honest swap, injection, drainer, agent key
   theft rejected **on-chain** (`ERR_OVER_TX_CAP`), freeze, then the **200-robot bench**
   (100 honest executed, 100 attacks stopped, F1 1.00).

URL options: `?lang=en` English page · `?mode=demo` force the scripted demo · `?t=42` freeze
frame · `?labels=1.7` bigger 3D labels · `?stack=1` each brick names the real software ·
`?invoice_to=testnet&invoice_near=0.1` invoice recipient and amount (for the testnet vault).
Keys: **space** = play/pause · **← / →** = ±2 s · **drag** = orbit · **wheel** = zoom ·
**double-click** = recentre. Details: [web/README.md](web/README.md).

### 8.2 A single interface

The 3D scene is the project's interface: there is no second dashboard. Everything is either in
the scene (vault state, certificate, decision timeline, emergency freeze, one-click requests),
or in the command line (`make demo`, `make attack`, `make sim`, `make mcp`).

---

## 8 bis. MCP server (AI agents)

The firewall is also exposed over **MCP** (JSON-RPC 2.0 on stdin/stdout): any agent (Claude
Desktop, Cursor, ZeroClaw, Codex) can ask for a transaction and get a reasoned verdict,
without ever holding the signing key.

```bash
make mcp                      # local mode, answers in English (default)
dryrun-gate mcp --lang fr      # answers in French
dryrun-gate mcp --network testnet --vault dryrun-vault-1112.testnet
```

Tools: `wallet_status`, `wallet_propose`, `wallet_freeze`, `wallet_explain`.

The answer says **where** the transaction was executed — `the local vault (simulation)`
locally, `NEAR testnet · tx <hash>` in testnet mode — and a **notification** (a message
without an `id`) gets no answer, as JSON-RPC 2.0 requires.

---

## 9. Acceptance criteria (all met)

- [x] **Rolling 24 h cap**: outputs bounded to 10 NEAR, including across an hour change (unit tests `tests::caps::i1_*`).
- [x] **Unknown contracts**: any call to an account missing from `known` is refused on-chain with `ERR_UNKNOWN_CONTRACT` (`tests::known::i3_*`).
- [x] **Watertight freeze**: the freeze key freezes in one transaction without a wallet; every later transaction is rejected with `ERR_FROZEN` (`tests::freeze::i2_*`).
- [x] **Exact simulation**: the simulated balance changes are identical to the real ones (+1 NEAR to bob).
- [x] **Trap corpus**: the 7 trap contracts (`drainer`, `skimmer`, `half-step`, `liar`, `approver`, `shapeshifter`, `two-face`) are blocked before signing (`make attack`, full simulator — Linux/macOS).
- [x] **Injection blocked**: "send 49 NEAR to mallory" is blocked by the firewall (R1) and rejected on-chain (`ERR_OVER_TX_CAP`).
- [x] **Ed25519 certificate (V1)**: on-chain cryptographic verification of the simulation certificate before execution.
- [x] **Post-check (V1)**: if the balance promised by the certificate is not reached on return, the vault freezes and marks the contract hostile (`tests/v1_post_check.rs`, real sandbox — Linux/macOS).
- [x] **Differential simulation (V1)**: R12 on the **default path** — every contract call with a deposit is replayed at one hundredth, in parallel (`two-face` blocked, 947 ‰ gap).
- [x] **Performance matrix**: `make sim` at 100 % precision on 200 synthetic robots.
- [x] **MCP server**: native integration for Claude / Cursor / ZeroClaw agents over JSON-RPC.
- [x] **Firewall on testnet**: verdict against the vault's real state (caps read over RPC), `ALLOW` signed and sent on-chain, transaction hash in the answer (section 6 ter).
- [x] **Simulator down**: every request is refused as a precaution ("any doubt blocks").
- [x] **Autonomy**: `make demo` and `make attack` run entirely locally, without secrets or network, in under 2 minutes (full simulator: Linux/macOS; light simulator: reduced effects, explicit `backend=stub` summary — see the platform note in section 2).

---

## 9 bis. Known limits and remaining work

Honesty beats a promise: here is what works, and what does not work yet.

| Topic | Status | Detail |
|---|---|---|
| Simulation (R1 to R12) | ✅ | The transaction is replayed in a sandbox, under the vault's identity. |
| V1 on-chain certificate | ✅ | Ed25519 signature verified by the contract, anti-replay nonces, freshness. |
| Corpus of 7 traps | ✅ | 7/7 blocked, 7/7 with the expected rule (`make attack`, full simulator — Linux/macOS). |
| 200-robot bench | ✅ | `make sim`: 100 % precision on the synthetic corpus. |
| MCP server | ✅ | JSON-RPC 2.0, 4 tools. |
| **Post-check** (`on_post_check`) | ✅ | Enabled: `call → on_done → balance read → on_post_check` within an agent's gas budget. A gap found → freeze + hostile marking (`tests/v1_post_check.rs`). |
| **Differential simulation** (R12) | ✅ | On the **default path**: every contract call with a deposit is replayed at one hundredth (`Gatekeeper::propose`). Both replays run in parallel, latency does not double. |
| Scope of R12 | ⚠️ | R12 is **directional**: it only fires when the small amount is proportionally better treated than the real amount (the two-face signature). A contract with a **fixed minimum deposit** (like `good-dex` here, 2 NEAR) cannot be replayed at one hundredth: R12 then gives no signal and the other rules decide. A legitimate fixed-price contract can be flagged — assumed, the threshold is 100 ‰ (the SPEC says 5 %, but an honest bonding curve already drifts by several percent between 2 NEAR and 0.02 NEAR). |
| **"Bare wallet" control** | ⚠️ | The control runs **after** the corpus, on the protected vault: it shows `ERR_NO_CERT` (the V1 defence), not the raw loss of funds. The raw loss is documented in the table of section 5 and shown by the simulation. |
| Light simulator (`--sim stub`) | ⚠️ | Used by default when `near-sandbox` is not available (e.g. on Windows): it returns no token delta, so token swaps end `BLOCK R8`, and `two-face` is stopped by R8 rather than R12. It also does not implement the V1 certificate, so `make demo` skips the key-theft check and `make attack` reports 4/7 — both exit 0 and name the simulator in their `SUMMARY` line (`backend=stub`). `make demo` / `make attack` with the full simulator give the results above (`backend=full`). |
| Vault on public testnet | ✅ | Deployed on `dryrun-vault-1112.testnet`: cap verified on-chain (5 NEAR refused, 0.1 NEAR executed). See section 6 bis. |
| Firewall wired to testnet | ✅ | `dryrun-gate mcp --network testnet --vault dryrun-vault-1112.testnet`: `status()` read over RPC, `ALLOW` requests signed and sent on-chain (tx [`BJqLxihw…`](https://explorer.testnet.near.org/transactions/BJqLxihwSfCpucK8VCHJPXKA7FmHZ2jUvoaivZUryVHa)), `BLOCK` sends nothing (sections 6 ter and 6 quater). |
| TEE enclave (MAX) | ❌ | Not started. The simulator key lives in the gatekeeper process. |

---

## 10. AI assistance & libraries

- **AI assistance**: architecture, Rust code, tests and documentation developed with the help of AI agents (ZeroClaw / hermes sub-agents, maxed) under the direction of GaiaSG94. All code was checked, compiled and tested on a local machine.
- **AI assistance (coding agents)**: **Claude Code** ("black lacquer" design, web HUD, 3D scene, documentation, demo video) and **opencode** (web implementation, live terminal, integration tests, MCP maintenance, testnet mode), under the direction of GaiaSG94. All code was reviewed, compiled and tested on a local machine.
- **Main libraries**:
  - `near-sdk` 5.30 (smart contracts)
  - `near-sandbox` 0.4 (local NEAR sandbox)
  - `near-primitives` / `near-crypto` 0.38 (transaction building and signing)
  - `serde`, `serde_json`, `tokio`, `borsh`.

---

## 10 bis. Repository history and provenance

- **All the code dates from the hackathon.** The first commit is from 10 November 2026 (13:43),
  after the 19 October announcement; no earlier code is reused. Full history:
  <https://github.com/GaiaSG94/dry-run/commits/main>.
- **The first two commits are large** (10/11: skeleton, types and rules, then contracts,
  simulator, gatekeeper and web, about 17,000 lines): this base was produced with the AI agents
  declared in section 10, then committed in two blocks. The rest of the history is incremental.
- **Submission tag**: `hackathon-submission` marks the submitted commit.

---

## 11. License

**MIT** — see [LICENSE](LICENSE).
