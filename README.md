# Dry Run web interface

A static page served by the gatekeeper: **no CDN, no external dependency, no image, no network
access**. Everything is 2D canvas + CSS.

The home page (`index.html`) is the 3D **"Board"** scene (design V9): a top-down view, like a
game board. The request follows a winding road through seven numbered steps: **1 AI agent →
2 firewall → 3 sandbox** (a dashed glass box holding the copies of the vault and of the
recipient) **→ 4 rules → 5 verdict → 6 vault** (the certificate lands above it) **→ 7 NEAR
chain** → recipient. Every element has its place; no volume crosses another.

Screen layout: at the top, **one white label**. At rest, it says what Dry Run does, with a
small line giving the mode (scripted demo or live mode). While a request plays, it says in one
sentence what is happening. Under it, **one button per request**. On the right, a **polished
glass terminal** logs every event; its header shows the mode. Every text of the scene sits flat
on a plate, above the 3D, placed so that nothing overlaps (`scene/labels.js`).

Visual identity: **"Black lacquer"**. Two colours only, pure black `#000000` and mint
`#3DFFB0`; everything else is white at some opacity. Colours live in `scene/theme.js`, element
positions in `scene/layout.js`.

There is **only one interface**: the 3D scene. Without JavaScript, the firewall is tested from
the command line (`make demo`, `make attack`, `make sim`).

## Two modes, detected automatically

| Mode | When | What happens |
|---|---|---|
| **LIVE** | the page is served by `dryrun-gate` (`/api/status` answers) | each button sends its request **to the real firewall**; the scene plays the steps with the real answer (verdict, rules, effects, execution) and the terminal shows the real HTTP calls |
| **DEMO** | page opened without a server (`file://`, GitHub Pages, or API unreachable) | the scripted film of `make demo` plays with the repository's real data; the buttons play the full film or each scenario alone |

```bash
# from the repository root — the NEAR sandbox is not needed for the interface
cargo run -p dryrun-gate -- serve --port 8787 --sim stub
# then open http://localhost:8787/?lang=en

# the same page wired to the vault deployed on NEAR testnet (real signatures)
DRYRUN_RPC_URL=https://rpc.testnet.fastnear.com \
  cargo run -p dryrun-gate -- serve --network testnet --vault dryrun-vault-1112.testnet
# then open http://localhost:8787/?lang=en&invoice_to=testnet&invoice_near=0.1
```

Options: `--port <n>` (default 8787), `--web-dir <folder>` (default `web`),
`--sim auto|real|stub` (`stub` plays on an in-memory vault, without `near-sandbox`) and
`--network testnet --vault <account>` (the deployed vault).

## URL parameters

| Parameter | Effect |
|---|---|
| `?lang=en` | English page (the default is French) — `scene/i18n.js` |
| `?mode=demo` | force the scripted demo even behind the server |
| `?t=42` | freeze frame at 42 s; `?play` starts automatically |
| `?labels=1.7` | scale every 3D label (video capture) |
| `?stack=1` | each 3D brick also names the real software behind it |
| `?invoice_to=…&invoice_near=…` | recipient and amount of the "pay invoice" button (an account that exists on testnet) |

## Files

| File | Role |
|---|---|
| `index.html` | the scene (loads the `scene/` modules) |
| `scene/i18n.js` | English layer (`?lang=en`): translates the DOM, the canvas labels and the freeze dialog |
| `scene/data.js` | the repository's real data: vault, R1–R12, 5 sandbox sub-steps, 7 certificate checks, 200-robot bench (seed 42), `make demo` scenarios |
| `scene/timeline.js` | timeline: beats built from the scenarios, `DR.at(t)` = full state at time *t* |
| `scene/renderer.js` | 3D canvas renderer: camera, projection, primitives, shaded glass volumes, depth haze, depth sorting |
| `scene/labels.js` | label layer: flat texts on plates, placed by priority without overlap, leader line when a label moves away from its element |
| `scene/shell.js` | assembly: canvas, the single label (pitch, then the step's sentence), request buttons, player, camera (drag, wheel, double-click), glass terminal |
| `scene/term.js` | live terminal on the right: logs every beat of the timeline (demo and live); `DR.term.log/sep/setMode` |
| `scene/scene.js` | the scene, driven by a theme and a layout: `DR.scene9(DR.THEME, DR.LAYOUT)` |
| `scene/layout.js` | the "Board" layout: position of each element, road, camera |
| `scene/theme.js` | the "Black lacquer" identity: colours, glass, welcome sentence. The only file to change for another identity |
| `scene/live.js` | LIVE mode: `/api/status`, `/api/log`, `/api/propose`, `/api/freeze`, and turning a real answer into a playable scenario |

## Do X → see Y (LIVE mode)

1. `cargo run -p dryrun-gate -- serve --port 8787 --sim stub`, open
   `http://localhost:8787/?lang=en` → the top label shows **"Live mode"**, the terminal header
   **`live · stub`** and the line `firewall connected · stub`; under the label, one button per
   real request, plus **▶ play the process** for the demo film.
2. **pay invoice · 1 Ⓝ** → the request enters the sandbox, the 5 sub-steps light up (the
   terminal details them one by one), the copy shows −1 NEAR, R1→R12 scroll with none firing →
   **ALLOW** → executed. On the testnet vault, the same request is signed and sent on-chain,
   and the 3D shows the transaction hash.
3. **injection 49 Ⓝ** → **BLOCK (R1)**: a white wave leaves the rules, the chain stays
   **intact**, nothing is signed; the MCP loop is shown on the **agent side** (the firewall
   never rephrases).
4. **drainer trap** → **BLOCK (R8)**: the funds go out, nothing comes back — the effects stop
   it, not an allow-list.
5. **two-face trap** → **BLOCK**: in LIVE mode it is stopped by R8 (see the limits below);
   the scripted film shows the **differential simulation (R12)** unmasking it.
6. **swap good-dex** → with the full simulator: **WARN (R5, first contact)** → the owner
   confirms (`confirm:true`) → **re-simulation** → **ALLOW** → the Ed25519 seal closes (7/7)
   → the transaction runs → **"sand = real"** with the real hash. With `--sim stub`, the light
   simulator returns no token delta, so this swap ends **BLOCK (R8)**.
7. **freeze the vault** → `POST /api/freeze` → the vault turns **FROZEN** and every later
   request is refused (`ERR_FROZEN`). Refused by design on the testnet vault.
8. The vault shown (balance, 24 h cap ring, journal ring) comes from `GET /api/status`,
   refreshed every 3 s.

## Do X → see Y (DEMO mode, no server)

1. Open <https://gaiasg94.github.io/dry-run/?lang=en>, or `web/index.html` directly → badge
   **"scripted demo"**, the terminal announces the demo mode.
2. Click **"▶ play the process"** (or the space bar) → the full `make demo` film: honest swap
   (WARN → approval → ALLOW), injection (BLOCK R1 + agent-side MCP loop), already-approved
   drainer (BLOCK R8), agent key theft (on-chain refusal `ERR_OVER_TX_CAP`), one-tap freeze,
   then the **200-robot bench** (seed 42: 100 honest executed, 100 attacks stopped,
   precision 1.00 · recall 1.00 · F1 1.00 · judgement 0 µs median / 2 µs p95).
3. Buttons under the label → play each scene separately.

## Controls

Drag = orbit (bounded) · wheel = zoom · double-click = recentre · space = play/pause ·
← / → = ±2 s.

## API used in LIVE mode

| Route | Use |
|---|---|
| `GET /api/status` | balance, caps, spent in 24 h, freeze, known contracts, backend |
| `GET /api/log` | journal length (ring of 100 entries) |
| `POST /api/propose` | the request; returns `verdict`, `rules`, `card_view` (NEAR and token deltas, receipts), `executed`, `note`, `cert` |
| `POST /api/freeze` | emergency freeze |

Amounts travel as **decimal yoctoNEAR strings** and are handled as `BigInt` in the page: no
float, no loss of precision.

## Known limits

- In LIVE mode, **R12 (differential simulation)** is not played: the HTTP API only exposes a
  simple `propose`. The one-hundredth divergence stays visible in DEMO mode (`make sim` data).
- In LIVE mode, the **agent key theft** is not replayed either (it would require calling the
  vault while bypassing the gatekeeper): it stays in the scripted film.
- The owner who confirms a `WARN` is, in the local demo, the same process as the gatekeeper (an
  assumed shortcut). In production, this approval comes from the owner's device.
- The freeze key can **only** freeze (invariant I7): the freeze button stays the only usable
  gesture when everything else is refused.
