/* dry-run · shell (design V9) — UNE seule étiquette blanche :
   au repos elle dit ce qu'est Dry Run ; en cours d'usage elle dit, en une phrase
   claire, ce qui se passe. Sous elle : les étiquettes des menaces (boutons).
   Plus d'étiquette « technologies », plus de ligne de mode séparée : le mode
   (démo / réel) est affiché dans l'en-tête du terminal. Le terminal est en verre
   poli (variante T.glass). API compatible live.js / term.js. */
(function () {
  "use strict";
  const DR = (window.DR = window.DR || {});

  /* ---------- phrases claires, une par étape ---------- */
  const amt = (d, u) => (d < 0n ? "− " : "+ ") + DR.fmt(d < 0n ? -d : d, 2) + " " + u;
  const STEP_SAY = [
    "on copie l’état réel du coffre et de la chaîne",
    "une clé jetable remplace la vraie clé",
    "la transaction est jouée comme si c’était le coffre",
    "on lit chaque reçu produit",
    "on compare avant / après : ce qui sort, ce qui rentre",
  ];
  DR.sentence = function (s) {
    const S = s.sc || s.scView, P = S && S.passes && S.passes.length ? S.passes[s.lastPass] : null;
    const rule = (id) => { const r = DR.rules.find((x) => x.id === id); return r ? id + " (" + r.k + ")" : id; };
    switch (s.ph) {
      case "arrive": return `<b>${S.tag}</b> — ${S.who} demande : ${S.say}`;
      case "send": return "La demande arrive au pare-feu. <b>Rien n’est signé</b> pour l’instant.";
      case "sandbox": return `<b>Bac à sable ${s.sub + 1}/5</b> — ${STEP_SAY[s.sub]}.` + (s.lastPass ? " (deuxième passage)" : "");
      case "effects": {
        const fx = (P ? P.effects : []).filter((e) => e.acc === "vault").map((e) => amt(e.d, e.u)).join(", ");
        return `Effets mesurés dans la copie, pour le coffre : <b>${fx || "aucun"}</b>.`;
      }
      case "rules": return P && P.rules.length ? `Les 12 règles examinent ces effets : <b>${P.rules.map(rule).join(", ")}</b> réagit.` : "Les 12 règles examinent ces effets : <b>aucune ne réagit</b>.";
      case "verdict":
        if (!P) return "";
        if (P.verdict === "ALLOW") return "<b>ALLOW</b> — les effets sont sûrs : la transaction peut être signée.";
        if (P.verdict === "WARN") return "<b>WARN</b> — premier contact avec ce contrat : le propriétaire doit l’approuver.";
        return `<b>BLOCK</b> — ${P.rules.map(rule).join(", ")} : la transaction ne sera jamais signée.`;
      case "approve": return "Le propriétaire approuve ce contrat (1 yocto). La demande est <b>rejouée</b>.";
      case "cert": return `Le simulateur signe un <b>certificat</b> ; le coffre le vérifie : ${DR.certChecks[s.sub].k} (${s.sub + 1}/7).`;
      case "chain": return "La <b>vraie</b> transaction s’exécute sur NEAR.";
      case "proof": return (S && S.proofLabel) || "Résultat réel = résultat simulé : <b>sand = réel</b>.";
      case "blocked": return "Rien n’est signé : <b>le coffre et la chaîne restent intacts</b>.";
      case "mcp": return "L’agent reçoit l’explication par MCP et peut reformuler. Le pare-feu, lui, ne reformule jamais.";
      case "bypass": return "Un voleur a la clé de l’agent et appelle le coffre <b>sans passer par le pare-feu</b>…";
      case "onchain": return `…le coffre refuse lui-même : <b>${S.onchainErr}</b>. Sans certificat valide, rien ne sort.`;
      case "freeze": return "Le propriétaire <b>gèle le coffre</b> d’un geste, depuis son téléphone (ERR_FROZEN).";
      case "bench": return "Banc de test : <b>200 demandes</b>, 100 honnêtes et 100 attaques.";
      case "end": return "<b>200 / 200</b> bien jugées : aucune attaque passée, aucun faux refus.";
      default: return s.caption || "";
    }
  };

  // verre poli : la plaque elle-même porte la lumière (reflet en biais, arête claire),
  // car derrière le terminal il n'y a que du noir — un simple flou ne se verrait pas.
  const GLASS = {
    poli: `background:linear-gradient(165deg,rgba(255,255,255,.20) 0%,rgba(255,255,255,.07) 22%,rgba(255,255,255,.035) 55%,rgba(61,255,176,.06) 100%);
           backdrop-filter:blur(18px) saturate(150%);border:1px solid rgba(255,255,255,.28);
           box-shadow:inset 0 1px 0 rgba(255,255,255,.55),inset 1px 0 0 rgba(255,255,255,.12),inset 0 0 40px rgba(61,255,176,.05),0 30px 80px -30px #000`,
    fume: `background:linear-gradient(170deg,rgba(61,255,176,.16) 0%,rgba(20,60,45,.30) 30%,rgba(6,18,14,.62) 100%);
           backdrop-filter:blur(26px) saturate(170%);border:1px solid rgba(61,255,176,.38);
           box-shadow:inset 0 1px 0 rgba(255,255,255,.35),inset 0 0 50px rgba(61,255,176,.08),0 30px 80px -30px #000`,
    givre: `background:radial-gradient(rgba(255,255,255,.11) .6px,transparent .8px) 0 0/3px 3px,linear-gradient(180deg,rgba(255,255,255,.17),rgba(255,255,255,.08));
           backdrop-filter:blur(34px) saturate(120%) brightness(1.15);border:1px solid rgba(255,255,255,.34);
           box-shadow:inset 0 1px 0 rgba(255,255,255,.6),inset 0 -30px 50px -30px rgba(255,255,255,.22),0 30px 80px -30px #000`,
  };

  DR.mount = function (conf) {
    const T = conf.theme || {};
    const root = document.documentElement.style;
    root.setProperty("--a", conf.accent); root.setProperty("--bg", conf.bg || "#000");
    const style = document.createElement("style");
    style.textContent = `
      html,body{margin:0;height:100%;background:var(--bg);overflow:hidden;color:#fff;font-family:${DR.SANS}}
      canvas#c{position:fixed;inset:0;width:100vw;height:100vh;display:block;cursor:grab}
      canvas#c:active{cursor:grabbing}
      .fx{position:fixed;inset:0;pointer-events:none;z-index:1;background:radial-gradient(ellipse at 50% 45%,transparent 55%,rgba(0,0,0,.55) 100%)}
      .hud{position:fixed;z-index:2;pointer-events:none}
      .hud a,.hud button{pointer-events:auto}
      .tr{top:20px;right:22px}
      .tr a{color:#fff;opacity:.6;text-decoration:none;border:1px solid rgba(255,255,255,.18);border-radius:99px;padding:7px 13px;font:500 11px/1 ${DR.MONO}}
      .top{top:16px;left:50%;transform:translateX(-50%);display:flex;flex-direction:column;align-items:center;gap:12px;width:min(94vw,820px)}
      #cap{background:#fff;border-radius:18px;padding:14px 26px;color:#0b4a33;font:500 20px/1.38 ${DR.SANS};text-align:center;
        max-width:100%;box-sizing:border-box;min-height:1.38em;box-shadow:0 12px 44px -12px rgba(0,0,0,.9);transition:background .3s}
      #cap b{font-weight:800;color:#01261a}
      #cap small{display:block;margin-top:8px;font:500 13.5px/1.35 ${DR.SANS};color:#3c6b58}
      #cap small i{display:inline-block;width:8px;height:8px;border-radius:50%;background:#9aa;margin-right:8px;vertical-align:1px}
      #cap small.live{color:#0b4a33;font-weight:600}#cap small.live i{background:var(--a);box-shadow:0 0 8px var(--a)}
      #cap small code{font:700 12.5px ${DR.MONO};background:color-mix(in srgb,var(--a) 22%,#fff);padding:1px 5px;border-radius:5px;color:#022b1c}
      #cap.v-BLOCK{box-shadow:0 0 0 3px #fff,0 12px 44px -12px rgba(0,0,0,.9)}
      .chips{display:flex;gap:8px;align-items:center;justify-content:center;flex-wrap:wrap}
      .chips .sepv{width:1px;height:26px;background:rgba(255,255,255,.25);margin:0 4px}
      .chip,#play{all:unset;pointer-events:auto;cursor:pointer;font:700 13px/1 ${DR.SANS};padding:11px 16px;border-radius:99px;
        color:var(--a);background:rgba(0,0,0,.55);backdrop-filter:blur(10px);box-shadow:inset 0 0 0 1.5px color-mix(in srgb,var(--a) 70%,transparent)}
      .chip:hover,#play:hover{background:var(--a);color:#001a10}
      .chip.on{background:var(--a);color:#001a10}
      .chip.danger{color:#fff;box-shadow:inset 0 0 0 1.5px rgba(255,255,255,.75)}
      .chip[disabled]{opacity:.4;cursor:wait}
      .bar{display:flex;gap:5px;width:min(80vw,460px)}
      .bar div{flex:1;height:2px;background:rgba(255,255,255,.14);overflow:hidden}
      .bar div i{display:block;height:100%;width:0;background:var(--a)}
      body #term{${GLASS[T.glass || "poli"]};border-radius:18px}
      body #term::before{content:"";position:absolute;inset:0;pointer-events:none;border-radius:inherit;
        background:linear-gradient(115deg,transparent 0 18%,rgba(255,255,255,.10) 22%,transparent 30% 100%)}
      body #term .th{border-bottom-color:rgba(255,255,255,.16)}
      #term .th span{font-weight:800;padding:4px 8px;border-radius:99px;border:1px solid currentColor}
      @media (max-width:760px){#cap{font-size:15px;padding:10px 14px}.chip,#play{font-size:12px;padding:9px 12px}.top{gap:8px;top:10px}}
    `;
    document.head.appendChild(style);
    document.body.insertAdjacentHTML("beforeend", `
      <canvas id="c"></canvas><div class="fx"></div>
      <div class="hud tr"><a href="${(conf.nav && conf.nav.next) || "index.html"}">${(conf.nav && conf.nav.nextL) || "planches"}</a></div>
      <div class="hud top" id="top">
        <div id="cap"></div>
        <div class="chips" id="chips"><button id="play" type="button">▶&nbsp; jouer le processus</button><span id="scen-menu" style="display:contents"></span></div>
        <div class="bar" id="bar"></div>
      </div>
      <span id="badge" hidden><span id="badge-t"></span></span>`);

    const canvas = document.getElementById("c");
    const R = DR.withLabels(DR.Renderer(canvas, conf));
    const bar = document.getElementById("bar"), cap = document.getElementById("cap"), btn = document.getElementById("play");
    const menu = document.getElementById("scen-menu"), top = document.getElementById("top");
    const q = new URLSearchParams(location.search);
    let t = q.has("t") ? parseFloat(q.get("t")) : 0;
    let mode = q.has("t") ? "pause" : "idle";
    if (q.has("play")) mode = "play";
    const setBtn = () => { btn.innerHTML = mode === "play" ? "❚❚&nbsp; pause" : mode === "end" ? "↺&nbsp; rejouer" : t > 0 ? "▶&nbsp; reprendre" : "▶&nbsp; jouer le processus"; };
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
      setBadge() {},
      // le mode (démo / réel) : pastille du terminal + petite ligne DANS l'étiquette, au repos
      setModeLine(html, live) { app.modeHtml = html; app.live = !!live; if (DR.term) DR.term.setMode(live ? "mode réel" : "démo scriptée", live); },
      setPlayLabel() { setBtn(); },
      onScenarios: rebuildBars, segs: [], bars: [],
      playScenario(i) { const b = DR.beats.find((x) => x.sc === i); if (b) { t = b.t0; mode = "play"; setBtn(); } },
      setScenarioMenu(items) {
        menu.innerHTML = "";
        (items || []).forEach((it) => {
          if (it.sep) { menu.insertAdjacentHTML("beforeend", '<span class="sepv"></span>'); return; }
          const b = document.createElement("button");
          b.type = "button"; b.textContent = it.label; b.className = "chip" + (it.danger ? " danger" : "");
          b.addEventListener("click", () => it.onSelect());
          menu.appendChild(b);
        });
      },
      setMenuBusy(on) { menu.querySelectorAll("button").forEach((b) => { b.disabled = on; }); },
    };
    app.demoMenu = () => app.setScenarioMenu(DR.SCRIPTED.map((S, i) => ({ label: S.tag, onSelect: () => app.playScenario(i) })));
    DR.app = app;
    btn.onclick = () => { if (mode === "play") mode = "pause"; else { if (mode === "end") t = 0; mode = "play"; } setBtn(); };
    window.addEventListener("keydown", (e) => {
      if (e.code === "Space" && e.target === document.body) { e.preventDefault(); btn.click(); }
      if (e.key === "ArrowRight") t = Math.min(DR.T, t + 2);
      if (e.key === "ArrowLeft") t = Math.max(0, t - 2);
    });
    let drag = null;
    canvas.addEventListener("pointerdown", (e) => { drag = { x: e.clientX, y: e.clientY, yaw: R.user.yaw, pitch: R.user.pitch }; canvas.setPointerCapture(e.pointerId); });
    canvas.addEventListener("pointermove", (e) => { if (!drag) return; R.user.yaw = DR.clamp(drag.yaw - (e.clientX - drag.x) * 0.004, -0.9, 0.9); R.user.pitch = DR.clamp(drag.pitch + (e.clientY - drag.y) * 0.003, -0.4, 0.5); });
    canvas.addEventListener("pointerup", () => { drag = null; });
    canvas.addEventListener("wheel", (e) => { e.preventDefault(); R.user.zoom = DR.clamp(R.user.zoom * (1 + Math.sign(e.deltaY) * 0.08), 0.6, 1.6); }, { passive: false });
    canvas.addEventListener("dblclick", () => { R.user = { yaw: 0, pitch: 0, zoom: 1 }; });
    R.resize();
    app.demoMenu(); rebuildBars(); setBtn();
    const base = Object.assign({}, R.cam);
    let last = performance.now(), clock = 0;
    function frame(now) {
      requestAnimationFrame(frame);
      if (canvas.clientWidth !== R.W || canvas.clientHeight !== R.H) R.resize();
      if (R.W < 2 || R.H < 2) return;
      const dt = Math.max(0, Math.min(0.05, (now - last) / 1000)); last = now; clock += dt;
      if (mode === "play") { t += dt; if (t >= DR.T) { t = DR.T; mode = "end"; setBtn(); } }
      const s = DR.at(t);
      s.mode = mode; s.clock = q.has("t") ? t : clock; s.dt = dt;
      const want = Object.assign({}, base, conf.cameraAt ? conf.cameraAt(s, R) : {});
      want.yaw = (want.yaw || 0) + Math.sin(s.clock * 0.13) * (T.drift == null ? 0.03 : T.drift);
      const kk = q.has("t") && mode === "pause" ? 1 : 1 - Math.pow(0.04, dt);
      ["yaw", "pitch", "dist", "fov"].forEach((key) => { R.cam[key] = DR.lerp(R.cam[key], want[key], kk); });
      R.cam.target = DR.mix3(R.cam.target, want.target || base.target, kk);
      const te = document.getElementById("term");
      R.padR = te && te.getClientRects().length ? Math.max(0, R.W - te.getBoundingClientRect().left + 12) : 0;
      R.padL = conf.padLeft ? conf.padLeft(R) : 0;
      const freeW = Math.max(200, R.W - R.padL - R.padR);
      top.style.left = (R.padL + freeW / 2) + "px";
      top.style.width = Math.min(freeW - 16, 820) + "px";
      R.padT = top.getBoundingClientRect().bottom + 6;
      R.basis();
      const ctx = R.ctx;
      ctx.setTransform(R.dpr, 0, 0, R.dpr, 0, 0);
      ctx.fillStyle = R.BG; ctx.fillRect(0, 0, R.W, R.H);
      if (conf.background) conf.background(R, s);
      conf.draw(R, s);
      R.flush();
      if (T.bloom && canvas.width > 8 && canvas.height > 8) {
        const bw = Math.ceil(canvas.width / 4), bh = Math.ceil(canvas.height / 4);
        if (!app.bl) app.bl = document.createElement("canvas");
        if (app.bl.width !== bw || app.bl.height !== bh) { app.bl.width = bw; app.bl.height = bh; }
        const b = app.bl.getContext("2d");
        b.globalCompositeOperation = "copy"; b.filter = "brightness(0.8) contrast(2.1) blur(3px)"; b.drawImage(canvas, 0, 0, bw, bh);
        ctx.save(); ctx.setTransform(1, 0, 0, 1, 0, 0); ctx.globalCompositeOperation = "lighter"; ctx.globalAlpha = T.bloom;
        ctx.drawImage(app.bl, 0, 0, canvas.width, canvas.height); ctx.restore();
      }
      R.solveLabels(); // les étiquettes, à plat, toujours par-dessus la 3D et jamais l'une sur l'autre
      // un message écrit par live.js (erreur réseau…) reste affiché 5 s avant la phrase d'étape
      if (app.capSet != null && cap.innerHTML !== app.capSet) { app.capHold = now + 5000; app.capSet = cap.innerHTML; }
      const html = mode === "idle" ? T.pitch + (app.modeHtml ? `<small class="${app.live ? "live" : ""}"><i></i>${app.modeHtml}</small>` : "") : DR.sentence(s);
      if (now > (app.capHold || 0) && cap.innerHTML !== html) { cap.innerHTML = html; app.capSet = cap.innerHTML; }
      cap.className = s.verdict && mode !== "idle" ? "v-" + s.verdict : "";
      menu.querySelectorAll(".chip").forEach((el, i) => el.classList.toggle("on", mode !== "idle" && DR.scenarios === DR.SCRIPTED && i === s.scI)); // en live, la liste jouée n'est pas celle des boutons
      const segI = s.scI < 0 ? app.segs.length - 1 : s.scI;
      app.bars.forEach((el, i) => {
        const bs = DR.beats.filter((b) => (i === app.segs.length - 1 && app.segs[i] === "banc" ? b.sc < 0 : b.sc === i));
        if (!bs.length) return;
        el.style.width = DR.clamp((t - bs[0].t0) / (bs[bs.length - 1].t1 - bs[0].t0)) * 100 + "%";
        el.parentNode.style.opacity = i === segI && mode !== "idle" ? "1" : ".7";
      });
      window.__dr = { t, ph: s.ph, mode };
    }
    requestAnimationFrame(frame);
    return R;
  };
})();
