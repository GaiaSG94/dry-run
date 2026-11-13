// Dry Run demo V5 (2:20, "Ada's day"): one 4K take of the real page (?lang=en&labels=1.7&stack=1),
// frame by frame in virtual time, the page clock keyframed on the voice.
//   0-65 s   the firewall wired to the TESTNET vault: invoice paid for real on-chain, injection blocked
//   65-90 s  a local firewall (stub): drainer, two-face, recorded key theft, freeze (freeze is refused on testnet)
//   90-124 s the testnet page at rest while NEAR is explained
// While the firewall answers (testnet RPC + a real signed transaction), no frame is taken: the wait is cut.
// Usage: PW_CORE=... CHROME=... node capture.js <testnetGate> <localGate>
const { chromium } = require(process.env.PW_CORE);
const fs = require("fs"), path = require("path"), { execFileSync } = require("child_process");
const HERE = __dirname, OUT = path.join(HERE, "frames");
const [GATE_T, GATE_L] = [process.argv[2] || "http://127.0.0.1:8801", process.argv[3] || "http://127.0.0.1:8802"];
const FPS = 30, W = 1920, H = 1080, TOTAL = 124.5;            // the end card (124.5-140) is built in HyperFrames
const Q = "/?lang=en&labels=1.7&stack=1&invoice_to=testnet&invoice_near=0.1";
const sleep = ms => new Promise(r => setTimeout(r, ms));

// live requests: click time, button, page-time keys after the firewall answered [video time, page time]
const LIVE = [
  { at: 22.8, btn: "/pay invoice/", keys: [[22.8, 0], [26.0, 3.6], [28.5, 7.8], [30.4, 9.6], [31.75, 11.4], [33.4, 12.8], [35.2, 14.6], [37.4, 16.8]] },
  { at: 41.2, btn: "/injection/", keys: [[41.2, 0], [45.3, 3.6], [53.9, 7.8], [55.6, 9.6], [58.1, 11.4], [59.5, 12.8], [60.6, 14.6], [63.5, 17.2]] },
  { at: 67.6, btn: "/drainer/", keys: [[67.6, 0], [69.3, 7.8], [70.2, 9.6], [71.15, 11.4], [72.4, 12.8]] },
  { at: 73.1, btn: "/two-face/", keys: [[73.1, 0], [75.2, 7.8], [76.0, 9.6], [76.9, 11.4], [78.2, 12.8]] },
];
const lerp = (keys, t) => {
  if (t <= keys[0][0]) return keys[0][1];
  for (let i = 1; i < keys.length; i++) if (t <= keys[i][0]) {
    const [a, x] = keys[i - 1], [b, y] = keys[i]; return x + (y - x) * (t - a) / (b - a);
  }
  return keys[keys.length - 1][1] + (t - keys[keys.length - 1][0]);
};

(async () => {
  fs.rmSync(OUT, { recursive: true, force: true }); fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
  let page = null;
  const P = js => page.evaluate(js);
  async function open(gate) {               // a fresh page (own clock); waits for the live connection
    const p = await (await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 })).newPage();
    p.on("dialog", d => d.accept());
    p.on("pageerror", e => console.error("page error:", e.message));
    await p.clock.install();
    await p.goto(gate + Q, { waitUntil: "load" });
    await p.waitForFunction(() => window.DR && DR.live && DR.app, null, { timeout: 90000 });
    await sleep(1500);
    await p.clock.pauseAt(Date.now() + 1000);
    if (page) await page.context().close();
    page = p;
  }
  const responses = () => P(`(document.getElementById('term-b')?.innerText.match(/response:/g) || []).length`);

  console.log("opening the testnet page…");
  await open(GATE_T);
  let k = 0, pageT = 0, live = null, liveT0 = 0, film = null;
  const done = new Set();
  async function step(vt) {
    for (const [i, c] of LIVE.entries()) if (!done.has("L" + i) && vt >= c.at) {
      done.add("L" + i);
      await page.waitForFunction(() => !document.querySelector("#chips button[disabled], button[disabled]"), null, { timeout: 120000 }).catch(() => {});
      const before = await responses();
      await P(`[...document.querySelectorAll('button')].find(b => ${c.btn}.test(b.innerText)).click()`);
      const t0 = Date.now();             // real time while the firewall answers: no frame is taken (cut)
      await page.waitForFunction(n => (document.getElementById('term-b')?.innerText.match(/response:/g) || []).length > n, before, { timeout: 180000, polling: 250 });
      console.log(`  ${c.btn} answered in ${((Date.now() - t0) / 1000).toFixed(1)} s (cut)`);
      live = c; liveT0 = pageT; film = null;
    }
    if (!done.has("local") && vt >= 65.2) { done.add("local"); console.log("switching to the local firewall…"); await open(GATE_L); pageT = 0; live = null; }
    if (!done.has("theft") && vt >= 79.6) {
      done.add("theft"); live = null;
      await P("document.getElementById('play').click()"); await P("DR.app.pause()");
      film = await P(`(() => { const f = (ph) => DR.beats.find(b => b.sc === 3 && b.ph === ph);
        return [[79.6, f("arrive").t0], [80.6, f("bypass").t0], [82.2, f("onchain").t0], [85.0, f("onchain").t1 - 0.05]]; })()`);
    }
    if (!done.has("freeze") && vt >= 85.5) {
      done.add("freeze"); film = null; live = null;
      await P(`[...document.querySelectorAll('button')].find(b => /freeze the vault/.test(b.innerText)).click()`);
    }
    if (!done.has("near") && vt >= 89.6) { done.add("near"); console.log("back to the testnet page at rest…"); await open(GATE_T); pageT = 0; live = null; film = null; }
    let dt = 1 / FPS;
    if (live) dt = Math.max(0.001, liveT0 + lerp(live.keys, vt + 1 / FPS) - pageT);
    if (film) await P(`DR.app.setT(${lerp(film, vt).toFixed(4)})`);
    pageT += dt;
    await page.clock.runFor(Math.max(1, Math.round(dt * 1000)));
  }
  while (k < Math.round(TOTAL * FPS)) {
    const vt = k / FPS;
    await step(vt);
    await page.screenshot({ path: path.join(OUT, String(k).padStart(5, "0") + ".jpg"), type: "jpeg", quality: 92 });
    k++;
    if (k % 300 === 0) console.log(`  ${(k / FPS).toFixed(0)} s`);
  }
  await browser.close();
  execFileSync("ffmpeg", ["-y", "-v", "error", "-framerate", String(FPS), "-i", path.join(OUT, "%05d.jpg"),
    "-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p", "-g", "15", "-movflags", "+faststart",
    path.join(HERE, "page4k.mp4")]);
  console.log(`done: page4k.mp4 (${(k / FPS).toFixed(1)} s)`);
})();
