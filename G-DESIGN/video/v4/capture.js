// Dry Run demo V4 (2:20): one continuous take of the real page (?lang=en&labels=1.7&stack=1) at
// 3840x2160, frame by frame in virtual time. The page clock speed is keyframed so each beat of the
// animation lands on its voice line. Live cases click the real buttons (the firewall answers);
// the honest swap and the key theft are the recorded film (make demo results), seeked on the voice.
// Usage: PW_CORE=... CHROME=... node capture.js <gateA> <gateB>
const { chromium } = require(process.env.PW_CORE);
const fs = require("fs"), path = require("path"), { execFileSync } = require("child_process");
const HERE = __dirname, OUT = path.join(HERE, "frames");
const [gateA, gateB] = [process.argv[2] || "http://127.0.0.1:8797", process.argv[3] || "http://127.0.0.1:8798"];
const FPS = 30, W = 1920, H = 1080, TOTAL = 140.0;
const Q = "/?lang=en&labels=1.7&stack=1";
const sleep = ms => new Promise(r => setTimeout(r, ms));

// live cases: [click time, [[video time, page time since click], ...]]
const LIVE = [
  { at: 32.3, btn: "/injection/", keys: [[32.3, 0], [37.85, 3.6], [46.5, 7.8], [48.0, 9.6], [51.3, 11.4], [52.6, 12.8], [53.6, 14.6], [55.9, 17.2]] },
  { at: 56.1, btn: "/drainer/", keys: [[56.1, 0], [59.6, 7.8], [63.8, 9.6], [66.3, 11.4], [67.6, 12.8]] },
  { at: 68.0, btn: "/two-face/", keys: [[68.0, 0], [72.0, 7.8], [73.3, 9.6], [74.7, 11.4], [75.9, 12.8]] },
];
// recorded film: [video time, film time] (swap = scenario 0; theft keys resolved at runtime)
const SWAP = [[76.5, 0], [79.8, 11.4], [81.5, 12.8], [83.4, 17.8], [86.9, 20.4], [87.7, 21.8], [89.0, 24.2], [90.1, 26.0], [90.9, 27.6]];
const lerp = (keys, t) => {
  if (t <= keys[0][0]) return keys[0][1];
  for (let i = 1; i < keys.length; i++) if (t <= keys[i][0]) {
    const [a, x] = keys[i - 1], [b, y] = keys[i]; return x + (y - x) * (t - a) / (b - a);
  }
  return keys[keys.length - 1][1] + (t - keys[keys.length - 1][0]);   // after the last key: real speed
};

(async () => {
  fs.rmSync(OUT, { recursive: true, force: true }); fs.mkdirSync(OUT, { recursive: true });
  const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
  let page = null;
  const P = js => page.evaluate(js);
  async function open(gate) {           // a fresh page (own clock) on a fresh firewall
    const p = await (await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: 2 })).newPage();
    p.on("dialog", d => d.accept());
    p.on("pageerror", e => console.error("page error:", e.message));
    await p.clock.install();
    await p.goto(gate + Q, { waitUntil: "load" });
    await p.waitForFunction(() => window.DR && DR.live && DR.app, null, { timeout: 20000 });
    await sleep(2000);
    await p.clock.pauseAt(Date.now() + 1000);
    if (page) await page.context().close();
    page = p;
  }
  await open(gateA);

  let k = 0, pageT = 0;                 // page clock advanced so far (s)
  let mode = "idle", live = null, liveT0 = 0, film = null, theft = null;
  const done = new Set();
  async function step(vt) {
    // actions
    for (const [i, c] of LIVE.entries()) if (!done.has("L" + i) && vt >= c.at) {
      done.add("L" + i); await P(`[...document.querySelectorAll('button')].find(b => ${c.btn}.test(b.innerText)).click()`);
      mode = "live"; live = c; liveT0 = pageT;
    }
    if (!done.has("swap") && vt >= SWAP[0][0]) {
      done.add("swap"); await P("document.getElementById('play').click()"); await P("DR.app.pause()");
      theft = await P(`(() => { const f = (ph) => DR.beats.find(b => b.sc === 3 && b.ph === ph);
        return [[90.95, f("arrive").t0], [93.6, f("bypass").t0], [96.4, f("onchain").t0], [99.0, f("onchain").t1 - 0.05]]; })()`);
      mode = "film"; film = SWAP.concat(theft);
    }
    if (!done.has("freeze") && vt >= 99.2) {
      done.add("freeze"); mode = "idle";
      await P(`[...document.querySelectorAll('button')].find(b => /freeze the vault/.test(b.innerText)).click()`);
    }
    if (!done.has("reload") && vt >= 104.5) {   // NEAR part: a fresh, unfrozen vault at rest (cut is invisible: rest → rest)
      done.add("reload"); mode = "idle"; await open(gateB); pageT = 0;
    }
    // page clock for this frame
    let dt = 1 / FPS;
    if (mode === "live") {
      const want = lerp(live.keys, vt + 1 / FPS);
      dt = Math.max(0.001, liveT0 + want - pageT);
    }
    if (mode === "film") await P(`DR.app.setT(${lerp(film, vt).toFixed(4)})`);
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
