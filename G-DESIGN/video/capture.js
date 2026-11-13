// Dry Run demo: one live page (?lang=en) driven shot by shot; every shot lasts exactly its voice line.
// Live shots click the real buttons (the firewall answers); the honest swap and the bench replay the
// recorded film (make demo results); the testnet card reads the real transactions from the public RPC.
// Usage: PW_CORE=<playwright-core dir> CHROME=<chrome.exe> node capture.js [gateUrl]
const { chromium } = require(process.env.PW_CORE);
const fs = require("fs"), path = require("path");
const HERE = __dirname, RAW = path.join(HERE, "raw");
const gate = process.argv[2] || "http://127.0.0.1:8792";
const LINES = JSON.parse(fs.readFileSync(path.join(HERE, "lines.json"), "utf8"));
const DUR = JSON.parse(fs.readFileSync(path.join(HERE, "voice", "durations.json"), "utf8"));
const GAP = 0.25, W = 1440, H = 810, BAND = 110, MINT = "#3DFFB0";
const VAULT = "dryrun-vault-1112.testnet";
const TX = { refused: "3YDpi4y9tTBJDBmu5VFCn1zFRPc1K3hBz3MHqQrqnjmB", executed: "HdZr1wuSSQSJQ3jcsHhqjzuEgP1opAfv6tdMdFTupck6" };
const sleep = ms => new Promise(r => setTimeout(r, ms));
const now = () => Date.now() / 1000;
const plain = t => t.replace(/\[[^\]]+\]\s*/g, "").replace(/\.\.\./g, "…").trim();
const esc = s => String(s).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

async function captions(browser) {
  const p = await browser.newPage({ viewport: { width: W, height: BAND } });
  for (const l of LINES) {
    const chip = l.chapter
      ? `<div style="flex:none;width:300px;display:flex;justify-content:flex-end"><span style="background:${MINT};color:#000;font:700 17px 'Segoe UI',system-ui,sans-serif;padding:7px 14px;border-radius:999px;white-space:nowrap">${esc(l.chapter)}</span></div>`
      : `<div style="flex:none;width:300px"></div>`;
    await p.setContent(`<html><body style="margin:0;background:#000;height:${BAND}px;display:flex;align-items:center;gap:26px;padding:0 40px 0 0;box-sizing:border-box;border-top:1px solid #1d2b25">
      ${chip}<div style="flex:1;color:#fff;font:600 23px/1.3 'Segoe UI Variable Display','Segoe UI',system-ui,sans-serif">${esc(plain(l.text))}</div></body></html>`);
    await p.screenshot({ path: path.join(RAW, `cap_${l.id}.png`) });
  }
  await p.close();
}

async function rpc(method, params) {
  for (const url of ["https://archival-rpc.testnet.fastnear.com", "https://archival-rpc.testnet.near.org", "https://rpc.testnet.near.org"]) {
    try {
      const r = await fetch(url, { method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }) });
      const d = await r.json();
      if (d.result) return d.result;
    } catch (e) { /* next endpoint */ }
  }
  return null;
}

async function testnetData() {   // the two execute() calls on the vault, read back from the chain
  const out = {};
  for (const [k, hash] of Object.entries(TX)) {
    const d = await rpc("EXPERIMENTAL_tx_status", { tx_hash: hash, sender_account_id: VAULT, wait_until: "NONE" });
    if (!d) return null;
    const call = d.transaction.actions[0].FunctionCall;
    const args = JSON.parse(Buffer.from(call.args, "base64").toString());
    const logs = d.receipts_outcome.flatMap(o => o.outcome.logs);
    const ev = logs.map(l => { try { return JSON.parse(l.replace(/^EVENT_JSON:/, "")); } catch (e) { return null; } }).find(Boolean);
    out[k] = { hash, method: call.method_name, amount: Number(BigInt(args.action.Transfer.amount) / 10n ** 20n) / 1e4,
      ok: d.status.SuccessValue !== undefined, event: ev ? ev.event + (ev.data.reason ? " · " + ev.data.reason : "") : "" };
  }
  const st = await rpc("query", { request_type: "call_function", finality: "final", account_id: VAULT, method_name: "status", args_base64: "e30=" });
  if (st) {
    const s = JSON.parse(Buffer.from(st.result).toString());
    out.status = { txCap: Number(BigInt(s.tx_cap) / 10n ** 22n) / 100, spent: Number(BigInt(s.spent_24h) / 10n ** 22n) / 100, frozen: s.frozen };
  }
  return out;
}

function cardHtml(d) {
  const row = (t, label) => `<div class="row ${t.ok ? "ok" : "bad"}">
      <div class="call">execute( Transfer → testnet, <b>${t.amount} NEAR</b> )</div>
      <div class="res">${t.ok ? "executed" : "refused on-chain"}<small>${esc(t.event)}</small></div>
      <div class="hash">${label} · tx ${t.hash.slice(0, 10)}…</div></div>`;
  return `<div id="vid-card"><div class="box">
    <div class="kick">NEAR testnet · read live from the public RPC</div>
    <h1>${VAULT}</h1>
    <div class="caps">vault contract · cap ${d.status ? d.status.txCap : 2} NEAR / tx · 10 NEAR / 24 h · spent today ${d.status ? d.status.spent : 0.1} NEAR</div>
    ${row(d.refused, "above the cap")}${row(d.executed, "under the cap")}
    <div class="foot">testnet.nearblocks.io/address/${VAULT}</div></div></div>`;
}

const CARD_CSS = `#vid-card,#vid-end{position:fixed;inset:0;z-index:99999;background:radial-gradient(ellipse at 50% 40%,#06221a 0%,#000 70%);display:flex;align-items:center;justify-content:center;font-family:'Segoe UI Variable Display','Segoe UI',system-ui,sans-serif;color:#fff;animation:vidIn .45s ease-out both}
@keyframes vidIn{from{opacity:0;transform:scale(1.02)}to{opacity:1;transform:none}}
#vid-card .box{width:980px}
#vid-card .kick{color:${MINT};font:700 18px Consolas,monospace;letter-spacing:.06em;text-transform:uppercase}
#vid-card h1{font:800 54px/1.1 Consolas,monospace;margin:14px 0 6px}
#vid-card .caps{color:#b9c8c1;font:500 21px Consolas,monospace;margin-bottom:34px}
#vid-card .row{display:grid;grid-template-columns:1fr 330px;gap:6px 24px;padding:22px 28px;margin:14px 0;border-radius:18px;border:2px solid;background:rgba(255,255,255,.04)}
#vid-card .row.bad{border-color:#ff5d6c}#vid-card .row.ok{border-color:${MINT}}
#vid-card .call{font:600 27px Consolas,monospace}
#vid-card .res{grid-row:span 2;align-self:center;text-align:right;font:800 30px 'Segoe UI',sans-serif}
#vid-card .bad .res{color:#ff5d6c}#vid-card .ok .res{color:${MINT}}
#vid-card .res small{display:block;font:600 17px Consolas,monospace;color:#cfd8d4;margin-top:6px}
#vid-card .hash{color:#8fa39b;font:500 17px Consolas,monospace}
#vid-card .foot{margin-top:28px;color:#8fa39b;font:500 19px Consolas,monospace}
#vid-end{flex-direction:column;text-align:center}
#vid-end .logo{width:120px;height:120px;border:7px solid ${MINT};border-radius:28px;transform:rotate(45deg);display:flex;align-items:center;justify-content:center;box-shadow:0 0 60px rgba(61,255,176,.35);margin-bottom:46px}
#vid-end .logo i{width:52px;height:52px;border:5px dashed ${MINT};border-radius:12px;opacity:.85}
#vid-end h1{font:800 96px/1 'Segoe UI Variable Display','Segoe UI',sans-serif;margin:0;letter-spacing:-.02em}
#vid-end h2{font:500 36px 'Segoe UI',sans-serif;color:#d7e3de;margin:18px 0 44px}
#vid-end .repo{font:700 32px Consolas,monospace;color:${MINT}}
#vid-end .cmd{font:500 22px Consolas,monospace;color:#a7b8b1;margin-top:14px}
#vid-end .near{margin-top:46px;font:700 20px 'Segoe UI',sans-serif;letter-spacing:.14em;color:#fff;border:2px solid #fff;border-radius:999px;padding:10px 26px}`;

const END_HTML = `<div id="vid-end"><div class="logo"><i></i></div><h1>Dry Run</h1>
  <h2>Every transaction happens twice. Only the safe one is signed.</h2>
  <div class="repo">github.com/GaiaSG94/dry-run</div>
  <div class="cmd">make demo · make attack · dryrun-gate serve</div>
  <div class="near">BUILT AT NEAR HACKATHON</div></div>`;

(async () => {
  fs.mkdirSync(RAW, { recursive: true });
  for (const f of fs.readdirSync(RAW)) { const p = path.join(RAW, f); if (fs.statSync(p).isFile()) fs.unlinkSync(p); }
  const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
  await captions(browser);
  const tn = await testnetData();
  if (!tn) { console.error("testnet RPC unreachable: aborting (the card must show real data)"); process.exit(1); }
  console.log("testnet:", JSON.stringify(tn));

  const ctx = await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 1 });
  const page = await ctx.newPage();
  page.on("dialog", d => d.accept());                     // the freeze confirmation
  page.on("pageerror", e => console.error("page error:", e.message));

  // Frame by frame in virtual time: the page clock (Date, timers, rAF, performance.now) is paused;
  // each frame advances it by exactly 1/30 s, then the frame is photographed. Smooth 30 fps whatever
  // the render speed of the machine. Real network calls (the firewall, the RPC) still answer in real time.
  const FPS = 30;
  let k = 0, vt = 0; const frames = [];
  async function frame() {
    const step = Math.round((k + 1) * 1000 / FPS) - Math.round(k * 1000 / FPS);
    await page.clock.runFor(step);
    const name = String(k).padStart(6, "0") + ".jpg";
    await page.screenshot({ path: path.join(RAW, name), type: "jpeg", quality: 90 });
    frames.push([name, vt]); k++; vt = k / FPS;
    if (k % 150 === 0) console.log(`  ${vt.toFixed(0)} s`);
  }
  const marks = {}, cuts = [];
  const now = () => vt;                                   // virtual seconds from here on
  async function prep(fn) { await fn(); }                 // set-up happens before frame 0
  async function play(id, actions = []) {
    marks[id] = vt; const end = vt + DUR[id] + GAP;
    const todo = actions.slice().sort((a, b) => a[0] - b[0]);
    while (vt < end) {
      while (todo.length && vt >= marks[id] + todo[0][0]) {
        const [, fn] = todo.shift();
        try { await fn(); } catch (e) { console.error(id, "action failed:", e.message); }
      }
      await frame();
    }
  }
  const P = js => page.evaluate(js);
  const button = re => P(`[...document.querySelectorAll('button')].find(b => ${re}.test(b.innerText)).click()`);
  // seek the recorded film to the start of a beat: scenario index, phase, pass
  const seek = (ph, pass, sc = 0) => P(`(() => { const b = DR.beats.find(x => x.sc === ${sc} && x.ph === "${ph}" && (${pass} === null || x.pass === ${pass}));
      if (b) { DR.app.setT(b.t0 + 0.02); DR.app.play(); } })()`);

  await prep(async () => {
    await page.clock.install();
    await page.goto(gate + "/?lang=en", { waitUntil: "load" });
    await page.waitForFunction(() => window.DR && DR.live && DR.app, null, { timeout: 20000 });
    await page.addStyleTag({ content: CARD_CSS + " *{transition:none!important}" });
    await sleep(2500);
    await page.clock.pauseAt(Date.now() + 1000);
  });

  await play("intro");
  await play("problem");
  await play("inject", [[0.3, () => button("/injection/")]]);
  await prep(async () => { await page.waitForFunction(() => !document.querySelector("button[disabled]"), null, { timeout: 15000 }).catch(() => {}); });
  await play("drainer", [[0.2, () => button("/drainer/")]]);
  // honest swap: the recorded film (make demo), seeked on the voice
  await play("swap", [[0.1, () => P("document.getElementById('play').click()")],
    [0.3, () => seek("arrive", null)], [4.1, () => seek("verdict", 0)], [6.4, () => seek("approve", 0)],
    [9.0, () => seek("effects", 1)], [11.6, () => seek("verdict", 1)], [12.8, () => seek("cert", 1)], [15.6, () => seek("proof", 1)]]);
  await play("freeze", [[0.1, () => P("DR.app.pause && DR.app.pause()")], [0.4, () => button("/freeze the vault/")]]);
  await play("testnet", [[0.05, () => P(`document.body.insertAdjacentHTML("beforeend", ${JSON.stringify(cardHtml(tn))})`)]]);
  await play("bench", [[0.05, async () => {
    await P("document.getElementById('play').click()");
    await P(`(() => { const b = DR.beats.find(x => x.ph === "bench"); DR.app.setT(b.t0 + 0.02); DR.app.play(); })()`);
    await P("document.getElementById('vid-card') && document.getElementById('vid-card').remove()");
  }]]);
  await play("cta", [[0.05, () => P(`document.body.insertAdjacentHTML("beforeend", ${JSON.stringify(END_HTML)})`)]]);
  for (let i = 0; i < FPS; i++) await frame();
  const stop = now();
  fs.writeFileSync(path.join(RAW, "frames.json"), JSON.stringify(frames));
  fs.writeFileSync(path.join(RAW, "marks.json"), JSON.stringify({ marks, cuts, stop }));
  console.log(`frames ${frames.length}, length ${(stop - frames[0][1]).toFixed(1)} s`);
  await browser.close();
})();
