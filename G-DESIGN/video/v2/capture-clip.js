// V2 product shot: the live page (?lang=en) answers the real prompt-injection request; rendered frame by
// frame in virtual time (page clock paused, +1/30 s per frame) -> frames/ then ffmpeg -> assets/inject.mp4.
// Usage: PW_CORE=... CHROME=... node capture-clip.js [gateUrl] [seconds]
const { chromium } = require(process.env.PW_CORE);
const fs = require("fs"), path = require("path"), { execFileSync } = require("child_process");
const HERE = __dirname, OUT = path.join(HERE, "frames");
const gate = process.argv[2] || "http://127.0.0.1:8793";
const SECONDS = parseFloat(process.argv[3] || "18.5"), FPS = 30, W = 1920, H = 1080;
const DPR = parseFloat(process.env.DPR || "1");          // 2 = 3840x2160 frames (the scene draws its canvas at devicePixelRatio, max 2)
const SPEED = parseFloat(process.env.SPEED || "1");      // page time per output second (0.75 = smooth slow motion)
const NAME = process.env.NAME || "inject.mp4";
const sleep = ms => new Promise(r => setTimeout(r, ms));

(async () => {
  fs.rmSync(OUT, { recursive: true, force: true }); fs.mkdirSync(OUT, { recursive: true });
  fs.mkdirSync(path.join(HERE, "assets"), { recursive: true });
  const browser = await chromium.launch({ executablePath: process.env.CHROME, headless: true });
  const page = await (await browser.newContext({ viewport: { width: W, height: H }, deviceScaleFactor: DPR })).newPage();
  page.on("pageerror", e => console.error("page error:", e.message));
  await page.clock.install();
  await page.goto(gate + "/?lang=en", { waitUntil: "load" });
  await page.waitForFunction(() => window.DR && DR.live && DR.app, null, { timeout: 20000 });
  await sleep(2000);
  await page.clock.pauseAt(Date.now() + 1000);
  let k = 0;
  const frame = async () => {
    await page.clock.runFor(Math.round((k + 1) * 1000 * SPEED / FPS) - Math.round(k * 1000 * SPEED / FPS));
    await page.screenshot({ path: path.join(OUT, String(k).padStart(5, "0") + ".jpg"), type: "jpeg", quality: 92 });
    k++;
  };
  for (let i = 0; i < 6; i++) await frame();                      // 0.2 s at rest
  await page.evaluate(() => [...document.querySelectorAll("button")].find(b => /injection/.test(b.innerText)).click());
  while (k < SECONDS * FPS) { await frame(); if (k % 90 === 0) console.log(`  ${(k / FPS).toFixed(0)} s`); }
  await browser.close();
  execFileSync("ffmpeg", ["-y", "-v", "error", "-framerate", String(FPS), "-i", path.join(OUT, "%05d.jpg"),
    "-c:v", "libx264", "-preset", "slow", "-crf", "14", "-pix_fmt", "yuv420p", "-g", "15", "-movflags", "+faststart",
    path.join(HERE, "assets", NAME)]);
  console.log(`done: assets/${NAME} (${(k / FPS).toFixed(1)} s)`);
})();
