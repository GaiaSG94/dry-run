/* dry-run · terminal live — colonne de droite : tout ce qui se passe, dit
   clairement et concisément, au moment où ça arrive. Chaque beat de la
   chronologie (démo OU réponse réelle du pare-feu) est journalisé ; live.js
   y ajoute les appels réseau. API : DR.term.log(html, kind), DR.term.sep(),
   DR.term.setMode(texte, live). Aucune dépendance. */
(function () {
  "use strict";
  const DR = window.DR;

  const style = document.createElement("style");
  style.textContent = `
    #term{position:fixed;right:22px;top:64px;bottom:24px;width:min(30vw,400px);z-index:3;
      display:flex;flex-direction:column;overflow:hidden;border-radius:14px;
      background:rgba(0,0,0,.68);border:1px solid color-mix(in srgb,var(--a,#3DFFB0) 25%,transparent);
      backdrop-filter:blur(14px);box-shadow:0 30px 80px -30px #000}
    #term .th{display:flex;align-items:center;gap:9px;padding:11px 14px;flex:none;
      border-bottom:1px solid rgba(255,255,255,.09);
      font:700 10.5px/1 ${DR.MONO};letter-spacing:.16em;text-transform:uppercase;color:rgba(255,255,255,.75)}
    #term .th i{width:7px;height:7px;border-radius:50%;background:var(--a,#3DFFB0);box-shadow:0 0 10px var(--a,#3DFFB0);animation:trmp 1.6s infinite}
    @keyframes trmp{50%{opacity:.35}}
    #term .th span{margin-left:auto;color:var(--a,#3DFFB0);opacity:.85;letter-spacing:.08em}
    #term .tb{flex:1;overflow-y:auto;padding:10px 13px;font:11.5px/1.6 ${DR.MONO};
      scrollbar-width:thin;scrollbar-color:rgba(255,255,255,.2) transparent}
    #term .tb::-webkit-scrollbar{width:6px}
    #term .tb::-webkit-scrollbar-thumb{background:rgba(255,255,255,.18);border-radius:3px}
    #term .ln{color:rgba(255,255,255,.78);word-break:break-word}
    #term .ln b{color:#fff;font-weight:700}
    #term .ln .ts{color:rgba(61,255,176,.5);margin-right:7px}
    #term .ln.ok{color:var(--a,#3DFFB0)}
    #term .ln.bad{color:#fff;font-weight:700}
    #term .ln.sys{color:rgba(255,255,255,.42)}
    #term .sepr{height:1px;background:rgba(255,255,255,.12);margin:9px 2px}
    @media (max-width:900px){#term{display:none}}
  `;
  document.head.appendChild(style);

  const el = document.createElement("div");
  el.id = "term";
  el.innerHTML = '<div class="th"><i></i>terminal<span id="term-mode">…</span></div><div class="tb" id="term-b"></div>';
  document.body.appendChild(el);
  const body = document.getElementById("term-b");

  const esc = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const scroll = () => { body.scrollTop = body.scrollHeight; };

  function log(text, kind) {
    const d = document.createElement("div");
    d.className = "ln" + (kind ? " " + kind : "");
    d.innerHTML = '<span class="ts">' + new Date().toTimeString().slice(0, 8) + "</span>" + text;
    body.appendChild(d);
    while (body.childElementCount > 240) body.removeChild(body.firstElementChild);
    scroll();
  }
  function sep() {
    const d = document.createElement("div");
    d.className = "sepr";
    body.appendChild(d);
    scroll();
  }
  function setMode(txt, live) {
    const m = document.getElementById("term-mode");
    m.textContent = txt;
    m.style.opacity = live ? "1" : ".6";
  }
  DR.term = { log, sep, setMode, esc };

  /* ---------- journalisation de la chronologie ---------- */
  const amt = (d, u) => (d < 0n ? "− " : "+ ") + DR.fmt(d < 0n ? -d : d, 2) + " " + u;

  function emitSub(s) {
    if (s.ph === "sandbox") log((s.beat.pass ? "re-simulation " : "sand ") + (s.sub + 1) + "/5 · " + esc(DR.sandboxSteps[s.sub].k));
    else if (s.ph === "cert") log("certificat " + (s.sub + 1) + "/7 · " + esc(DR.certChecks[s.sub].k));
  }

  function emitBeat(s) {
    const S = s.sc;
    switch (s.ph) {
      case "arrive":
        sep();
        log("<b>── " + esc(S.tag) + " ──</b>");
        log(esc(S.who) + " : " + esc(S.say), "sys");
        break;
      case "send": log("demande → pare-feu"); break;
      case "sandbox": emitSub(s); break;
      case "effects":
        log("effets observés :");
        (s.effects || []).forEach((e) => log(esc(e.acc) + " " + amt(e.d, e.u), e.d > 0n ? "ok" : ""));
        break;
      case "rules": {
        const P = S && S.passes.length ? S.passes[s.lastPass] : null;
        const ids = P ? P.rules : [];
        if (!ids.length) log("R1 → R12 : rien à signaler", "ok");
        else ids.forEach((id) => {
          const r = DR.rules.find((x) => x.id === id);
          log("règle " + id + (r ? " · " + esc(r.k) + " → " + r.v : ""), "bad");
        });
        break;
      }
      case "verdict": {
        const P = S && S.passes.length ? S.passes[s.lastPass] : null;
        const v = s.verdict || "";
        const ids = P && P.rules.length ? " (" + P.rules.join(" ") + ")" : "";
        log("verdict : <b>" + esc(v) + "</b>" + ids, v === "ALLOW" ? "ok" : v === "BLOCK" ? "bad" : "");
        break;
      }
      case "approve": log("propriétaire : approbation · 1 yocto"); break;
      case "cert": emitSub(s); break;
      case "chain": log("exécution réelle sur NEAR…"); break;
      case "proof": log(esc(S && S.proofLabel ? S.proofLabel : "sand = réel ✓"), "ok"); break;
      case "blocked": log("rien n'est signé · la chaîne reste intacte", "bad"); break;
      case "mcp": log("côté agent : boucle MCP · wallet_explain", "sys"); break;
      case "bypass": log("contournement du pare-feu (sans certificat)", "bad"); break;
      case "onchain": log("refus on-chain : <b>" + esc(S.onchainErr) + "</b>", "bad"); break;
      case "freeze":
        sep();
        log("<b>── gel d'un geste ──</b>");
        log("clé de gel → le coffre est FROZEN (ERR_FROZEN)", "bad");
        break;
      case "bench": sep(); log("banc : 200 robots · graine 42", "sys"); break;
      case "end": log("<b>200 / 200 · F1 1,00 · 2 µs p95</b>", "ok"); break;
      default: break;
    }
  }

  let lastBi = -1, lastSub = -1, lastScI = -2, lastT = 0;
  function tick() {
    requestAnimationFrame(tick);
    if (!DR.app || !DR.beats || !DR.beats.length) return;
    const t = DR.app.t;
    if (t < lastT - 0.5) { lastBi = -1; lastSub = -1; lastScI = -2; } // retour au début
    lastT = t;
    if (DR.app.mode === "idle") return;
    const s = DR.at(t);
    if (s.bi !== lastBi || s.scI !== lastScI) emitBeat(s);
    else if (s.sub !== lastSub && (s.ph === "sandbox" || s.ph === "cert")) emitSub(s);
    lastBi = s.bi; lastScI = s.scI; lastSub = s.sub;
  }
  requestAnimationFrame(tick);

  log("terminal prêt", "sys");
  log("en attente — cliquez sur ▶ ou choisissez un scénario", "sys");
})();
