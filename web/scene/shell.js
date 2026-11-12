/* dry-run · shell V8 — montage de la scène : canvas plein écran, HUD minimal
   (marque + badge, aide, étiquette blanche d'événements, étiquette technologies,
   menu déroulant « scénario », lecteur), caméra glisser/molette/double-clic,
   halo lumineux. Compatible live.js et term.js. ids : #cap #bar #scen-btn
   #scen-menu #play #badge #badge-t. */
(function () {
  "use strict";
  const DR = (window.DR = window.DR || {});

  DR.mount = function (conf) {
    const T = conf.theme || {};
    const root = document.documentElement.style;
    root.setProperty("--a", conf.accent); root.setProperty("--bg", conf.bg || "#05070a");
    root.setProperty("--ink", T.ink || "#ffffff");
    const fam = (k) => (k === "serif" ? DR.SERIF : k === "mono" ? DR.MONO : k === "cond" ? DR.COND : DR.SANS);
    DR.COND = DR.COND || '"Bahnschrift", "DIN Alternate", "Arial Narrow", sans-serif';
    const FB = fam(T.font), FD = T.display ? fam(T.display) : FB;
    const condCss = (T.display || T.font) === "cond" ? "#cap,.tl b,#play{font-stretch:condensed}" : "";
    const up = T.upper ? "uppercase" : "none";
    const style = document.createElement("style");
    style.textContent = `
      html,body{margin:0;height:100%;background:var(--bg);overflow:hidden;color:var(--ink);font-family:${FB}}
      canvas#c{position:fixed;inset:0;width:100vw;height:100vh;display:block;cursor:grab}
      canvas#c:active{cursor:grabbing}
      .fx{position:fixed;inset:0;pointer-events:none;z-index:1}
      .vig{background:radial-gradient(ellipse at 50% 45%,transparent 55%,rgba(0,0,0,.55) 100%)}
      .scan{background:repeating-linear-gradient(0deg,rgba(0,0,0,.22) 0 1px,transparent 1px 3px);mix-blend-mode:multiply}
      .grain{opacity:.06;background-image:radial-gradient(rgba(255,255,255,.9) .5px,transparent .6px);background-size:3px 3px}
      .hud{position:fixed;z-index:2;pointer-events:none;letter-spacing:.02em}
      .hud a,.hud button{pointer-events:auto}
      .tl{top:22px;left:26px;display:flex;flex-direction:column;gap:8px}
      .tl b{font:${T.brandWeight || 800} 20px/1 ${FD};letter-spacing:${T.upper ? ".28em" : "-.01em"};text-transform:${up}}
      .tl b i{font-style:normal;color:var(--a)}
      .tl span{opacity:.5;font-size:13px}
      .badge{display:inline-flex;align-items:center;gap:7px;font:600 10px/1 ${DR.MONO};letter-spacing:.14em;text-transform:uppercase;
        border:1px solid color-mix(in srgb,var(--ink) 20%,transparent);border-radius:${T.square ? "2px" : "99px"};padding:5px 10px;opacity:.75;width:max-content}
      .badge i{width:6px;height:6px;border-radius:50%;background:currentColor;opacity:.5}
      .badge.live{color:var(--a);border-color:var(--a);opacity:1}
      .tr{top:20px;right:22px;display:flex;gap:6px}
      .tr a{color:inherit;opacity:.6;text-decoration:none;border:1px solid color-mix(in srgb,var(--ink) 18%,transparent);
        border-radius:${T.square ? "2px" : "99px"};padding:7px 13px;backdrop-filter:blur(10px);font:500 11px/1 ${DR.MONO}}
      .tr a:hover{opacity:1;border-color:var(--a)}
      .bc{left:50%;bottom:24px;transform:translateX(-50%);display:flex;flex-direction:column;align-items:center;gap:12px;width:min(94vw,900px)}
      .tags{display:flex;gap:10px;align-items:center;justify-content:center;flex-wrap:wrap}
      .tag{background:#fff;border-radius:14px;padding:10px 20px;color:color-mix(in srgb,var(--a) 45%,#022b1c);
        font:${T.capWeight || 600} 16px/1.3 ${FD};text-align:center;max-width:min(72vw,580px);
        box-shadow:0 10px 40px -10px rgba(0,0,0,.85),0 0 0 1px rgba(255,255,255,.25);text-transform:${up}}
      .tag em{font-style:normal;font-weight:800;color:color-mix(in srgb,var(--a) 62%,#01261a)}
      .tag.tech{display:flex;align-items:center;gap:7px;border-radius:99px;padding:10px 16px;white-space:nowrap;
        font:700 12px/1 ${DR.MONO};letter-spacing:.03em;text-transform:none}
      .tag.tech svg{width:14px;height:14px;fill:currentColor;flex:none}
      .bar{display:flex;gap:5px;width:min(80vw,460px)}
      .bar div{flex:1;height:2px;background:color-mix(in srgb,var(--ink) 14%,transparent);overflow:hidden}
      .bar div i{display:block;height:100%;width:0;background:var(--a);box-shadow:0 0 8px var(--a)}
      .ctl{display:flex;gap:10px;align-items:center}
      .dd{position:relative}
      #scen-btn{all:unset;pointer-events:auto;cursor:pointer;font:600 11px/1 ${DR.MONO};letter-spacing:.1em;text-transform:uppercase;
        padding:12px 18px;border:1px solid color-mix(in srgb,var(--ink) 22%,transparent);border-radius:99px;opacity:.85;transition:border-color .15s,color .15s}
      #scen-btn:hover{border-color:var(--a);color:var(--a)}
      #scen-btn[disabled]{opacity:.4;cursor:wait}
      .dd-menu{pointer-events:auto;position:absolute;bottom:calc(100% + 12px);left:50%;transform:translateX(-50%);
        min-width:255px;background:#fff;border-radius:16px;padding:7px;display:none;flex-direction:column;gap:2px;
        box-shadow:0 24px 60px -15px rgba(0,0,0,.85)}
      .dd.open .dd-menu{display:flex}
      .dd-menu button{all:unset;cursor:pointer;pointer-events:auto;font:600 13px/1.3 ${FB};color:color-mix(in srgb,var(--a) 45%,#022b1c);padding:9px 12px;border-radius:10px}
      .dd-menu button:hover{background:color-mix(in srgb,var(--a) 18%,#fff)}
      .dd-menu button[disabled]{opacity:.4;cursor:wait}
      .dd-menu button.danger{color:#a03030}
      .dd-menu hr{border:0;border-top:1px solid rgba(0,0,0,.1);margin:5px 8px}
      #play{all:unset;pointer-events:auto;cursor:pointer;display:flex;align-items:center;gap:12px;padding:16px 36px;
        border-radius:${T.square ? "3px" : "99px"};background:var(--a);color:var(--bg);font:800 15px/1 ${FD};letter-spacing:.08em;text-transform:${up};
        box-shadow:0 0 0 1px rgba(255,255,255,.18),0 14px 60px -12px var(--a);transition:transform .2s}
      #play:hover{transform:translateY(-1px) scale(1.02)}
      #play.on{background:color-mix(in srgb,var(--ink) 6%,transparent);color:var(--ink);box-shadow:0 0 0 1px color-mix(in srgb,var(--ink) 25%,transparent);backdrop-filter:blur(12px)}
      .sw{display:inline-flex;align-items:center;gap:8px;font:600 11px/1 ${DR.MONO};opacity:.75}
      .sw u{width:12px;height:12px;border-radius:50%;background:var(--a);box-shadow:0 0 10px var(--a)}
      .lb{position:fixed;left:0;right:0;height:9vh;background:#000;z-index:1}
      ${T.ghost ? "#play{background:transparent;color:var(--a);box-shadow:inset 0 0 0 1.5px var(--a)}#play.on{box-shadow:inset 0 0 0 1px color-mix(in srgb,var(--ink) 30%,transparent)}" : ""}
      ${T.capPx ? "#cap{font-size:" + T.capPx + "px}" : ""}
      ${condCss}
      @media (max-width:760px){.tl span{display:none}.tag{font-size:13px;padding:8px 14px;max-width:88vw}.tag.tech{display:none}#play{padding:13px 24px;font-size:13px}.tl b{font-size:16px}.bc{gap:9px}}
    `;
    document.head.appendChild(style);
    const nav = conf.nav || { next: "README.md", nextL: "aide" };
    document.body.insertAdjacentHTML("beforeend", `
      <canvas id="c"></canvas>
      ${T.letterbox ? '<div class="lb" style="top:0"></div><div class="lb" style="bottom:0"></div>' : ""}<div class="fx vig"></div>${T.scan ? '<div class="fx scan"></div>' : ""}${T.grain ? '<div class="fx grain"></div>' : ""}
      <div class="hud tl">
        <b>dry<i>·</i>run</b>
        ${conf.idea ? `<span>${conf.idea}</span>` : ""}
        <div class="badge" id="badge"><i></i><span id="badge-t">démo scriptée</span></div>
        ${T.swatch ? `<div class="sw"><u></u>${T.swatch}</div>` : ""}
      </div>
      <div class="hud tr">${nav.links || `<a href="${nav.next}">${nav.nextL}</a>`}</div>
      <div class="hud bc">
        <div class="tags">
          <div class="tag" id="cap"></div>
          ${conf.tech ? `<div class="tag tech" id="tech">${conf.tech}</div>` : ""}
        </div>
        <div class="bar" id="bar"></div>
        <div class="ctl">
          <div class="dd" id="dd"><button id="scen-btn" type="button">scénario&nbsp;▾</button><div class="dd-menu" id="scen-menu"></div></div>
          <button id="play" type="button">▶&nbsp; jouer le processus</button>
        </div>
      </div>`);

    const canvas = document.getElementById("c");
    const R = DR.Renderer(canvas, conf);
    const bar = document.getElementById("bar"), cap = document.getElementById("cap"), btn = document.getElementById("play");
    const dd = document.getElementById("dd"), scenBtn = document.getElementById("scen-btn"), menu = document.getElementById("scen-menu");
    const q = new URLSearchParams(location.search);
    let t = q.has("t") ? parseFloat(q.get("t")) : 0;
    let mode = q.has("t") ? "pause" : "idle";
    if (q.has("play")) mode = "play";
    const setBtn = () => {
      btn.classList.toggle("on", mode === "play");
      btn.innerHTML = mode === "play" ? "❚❚&nbsp; pause" : mode === "end" ? "↺&nbsp; rejouer" : t > 0 ? "▶&nbsp; reprendre" : conf.playLabel || "▶&nbsp; jouer le processus";
    };
    function rebuildBars() {
      const segs = DR.scenarios.map((S) => S.tag);
      if (DR.beats.some((b) => b.sc < 0)) segs.push("banc");
      bar.innerHTML = segs.map((k) => `<div title="${k}"><i></i></div>`).join("");
      app.segs = segs; app.bars = [...bar.querySelectorAll("i")];
    }
    const app = {
      R, conf, get t() { return t; }, get mode() { return mode; },
      setT(v) { t = DR.clamp(v, 0, DR.T); },
      play() { if (mode === "end") t = 0; mode = "play"; setBtn(); },
      pause() { mode = "pause"; setBtn(); },
      stop() { mode = "idle"; t = 0; setBtn(); },
      setBadge(text, live) { document.getElementById("badge-t").textContent = text; document.getElementById("badge").classList.toggle("live", !!live); },
      setPlayLabel(text) { conf.playLabel = text; setBtn(); },
      onScenarios: rebuildBars, segs: [], bars: [],
      playScenario(i) { const b = DR.beats.find((x) => x.sc === i); if (b) { t = b.t0; mode = "play"; setBtn(); } },
      setScenarioMenu(items) {
        menu.innerHTML = "";
        (items || []).forEach((it) => {
          if (it.sep) { menu.insertAdjacentHTML("beforeend", "<hr>"); return; }
          const b = document.createElement("button");
          b.type = "button"; b.textContent = it.label;
          if (it.danger) b.className = "danger";
          b.addEventListener("click", () => { dd.classList.remove("open"); it.onSelect(); });
          menu.appendChild(b);
        });
      },
      setMenuBusy(on) { scenBtn.disabled = on; menu.querySelectorAll("button").forEach((b) => { b.disabled = on; }); },
    };
    app.demoMenu = function () {
      const items = [{ label: "▶ processus complet", onSelect: () => { app.setT(0); app.play(); } }];
      DR.SCRIPTED.forEach((S, i) => items.push({ label: S.tag, onSelect: () => app.playScenario(i) }));
      app.setScenarioMenu(items);
    };
    DR.app = app;
    btn.onclick = () => { if (mode === "play") mode = "pause"; else { if (mode === "end") t = 0; mode = "play"; } setBtn(); };
    scenBtn.addEventListener("click", (e) => { e.stopPropagation(); dd.classList.toggle("open"); });
    menu.addEventListener("click", (e) => e.stopPropagation());
    document.addEventListener("click", () => dd.classList.remove("open"));
    window.addEventListener("keydown", (e) => {
      if (e.code === "Space" && e.target === document.body) { e.preventDefault(); btn.click(); }
      if (e.key === "ArrowRight") t = Math.min(DR.T, t + 2);
      if (e.key === "ArrowLeft") t = Math.max(0, t - 2);
      if (e.key === "Escape") dd.classList.remove("open");
    });
    let drag = null;
    canvas.addEventListener("pointerdown", (e) => { drag = { x: e.clientX, y: e.clientY, yaw: R.user.yaw, pitch: R.user.pitch }; canvas.setPointerCapture(e.pointerId); });
    canvas.addEventListener("pointermove", (e) => {
      if (!drag) return;
      R.user.yaw = DR.clamp(drag.yaw - (e.clientX - drag.x) * 0.004, -0.9, 0.9);
      R.user.pitch = DR.clamp(drag.pitch + (e.clientY - drag.y) * 0.003, -0.4, 0.5);
    });
    canvas.addEventListener("pointerup", () => { drag = null; });
    canvas.addEventListener("wheel", (e) => { e.preventDefault(); R.user.zoom = DR.clamp(R.user.zoom * (1 + Math.sign(e.deltaY) * 0.08), 0.6, 1.6); }, { passive: false });
    canvas.addEventListener("dblclick", () => { R.user = { yaw: 0, pitch: 0, zoom: 1 }; });
    window.addEventListener("resize", R.resize);
    R.resize();
    if (conf.init) conf.init(R);
    app.demoMenu(); rebuildBars(); setBtn();
    const base = Object.assign({}, R.cam);
    let last = performance.now(), clock = 0;
    const drift = T.drift == null ? 0.05 : T.drift;
    function frame(now) {
      requestAnimationFrame(frame); // relancée d'abord : une erreur dans une image n'arrête jamais l'animation
      // le canvas suit la taille réelle de la page, même si elle a changé sans événement « resize »
      if (canvas.clientWidth !== R.W || canvas.clientHeight !== R.H) R.resize();
      if (R.W < 2 || R.H < 2) return; // page masquée : rien à dessiner
      // dt jamais négatif : sur une machine lente, l'horodatage de la 1re image peut précéder `last`,
      // et un dt négatif projetait la caméra très loin (scène vide ou vue de travers).
      const dt = Math.max(0, Math.min(0.05, (now - last) / 1000)); last = now; clock += dt;
      if (mode === "play") { t += dt; if (t >= DR.T) { t = DR.T; mode = "end"; setBtn(); } }
      const s = DR.at(t);
      s.mode = mode; s.clock = q.has("t") ? t : clock; s.dt = dt;
      const want = Object.assign({}, base, conf.cameraAt ? conf.cameraAt(s, R) : {});
      want.yaw = (want.yaw || 0) + Math.sin(s.clock * 0.13) * drift; // caméra vivante : lente dérive orbitale
      // instant figé (?t=) : la caméra se pose directement, sans attendre le lissage
      const kk = q.has("t") && mode === "pause" ? 1 : 1 - Math.pow(T.camEase || 0.04, dt);
      ["yaw", "pitch", "dist", "fov"].forEach((key) => { R.cam[key] = DR.lerp(R.cam[key], want[key], kk); });
      R.cam.target = DR.mix3(R.cam.target, want.target || base.target, kk);
      // marges réservées : le terminal (s'il est affiché) à droite, le rail de mots à gauche.
      // Sans cela le terminal recouvrait le destinataire et les chiffres du banc.
      const te = document.getElementById("term");
      R.padR = te && te.getClientRects().length ? Math.max(0, R.W - te.getBoundingClientRect().left + 12) : 0;
      R.padL = conf.padLeft ? conf.padLeft(R) : 0;
      R.basis();
      const ctx = R.ctx;
      ctx.setTransform(R.dpr, 0, 0, R.dpr, 0, 0);
      ctx.fillStyle = R.BG; ctx.fillRect(0, 0, R.W, R.H);
      if (conf.background) conf.background(R, s);
      conf.draw(R, s);
      R.flush();
      if (conf.overlay) conf.overlay(R, s);
      // halo lumineux : l'image réduite, seuillée et floutée est ré-additionnée.
      // Jamais sur un canvas de taille nulle (page chargée en arrière-plan) : drawImage lèverait une erreur.
      if (T.bloom && canvas.width > 8 && canvas.height > 8) {
        const bw = Math.ceil(canvas.width / 4), bh = Math.ceil(canvas.height / 4);
        if (!app.bl) app.bl = document.createElement("canvas");
        if (app.bl.width !== bw || app.bl.height !== bh) { app.bl.width = bw; app.bl.height = bh; }
        const b = app.bl.getContext("2d");
        b.globalCompositeOperation = "copy";
        b.filter = "brightness(0.8) contrast(2.1) blur(3px)"; // seuil : seules les zones lumineuses rayonnent
        b.drawImage(canvas, 0, 0, bw, bh);
        ctx.save(); ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalCompositeOperation = "lighter"; ctx.globalAlpha = T.bloom;
        ctx.drawImage(app.bl, 0, 0, canvas.width, canvas.height); ctx.restore();
      }
      cap.innerHTML = mode === "idle" ? conf.hello || "" : s.caption.replace(/(ALLOW|WARN|BLOCK|ERR_[A-Z_]+|R\d+)/g, "<em>$1</em>");
      const segI = s.scI < 0 ? app.segs.length - 1 : s.scI;
      app.bars.forEach((el, i) => {
        const bs = DR.beats.filter((b) => (i === app.segs.length - 1 && app.segs[i] === "banc" ? b.sc < 0 : b.sc === i));
        if (!bs.length) return;
        el.style.width = DR.clamp((t - bs[0].t0) / (bs[bs.length - 1].t1 - bs[0].t0)) * 100 + "%";
        el.parentNode.style.opacity = i === segI && mode !== "idle" ? "1" : ".7";
      });
      window.__dr = { t, ph: s.ph, mode, live: !!DR.live };
    }
    requestAnimationFrame(frame);
    return R;
  };
})();
