/* dry-run · chronologie : un seul bouton joue tout le processus.
   Les beats sont construits à partir des scénarios (démo scriptée OU réponse
   réelle du pare-feu), puis DR.at(t) donne l'état complet à l'instant t. */
(function () {
  "use strict";
  const DR = (window.DR = window.DR || {});

  /* ---------- phases ---------- */
  DR.PH = {
    arrive: "l’agent prépare sa demande",
    send: "la demande part vers le pare-feu",
    sandbox: "bac à sable — hors chaîne",
    effects: "effets observés",
    rules: "règles R1 → R12",
    verdict: "verdict",
    approve: "le propriétaire approuve — 1 yocto",
    cert: "certificat Ed25519",
    chain: "exécution réelle sur NEAR",
    proof: "sand = réel",
    blocked: "rien n’est signé — la chaîne reste intacte",
    mcp: "boucle côté agent (MCP) — hors pare-feu",
    bypass: "le voleur contourne le pare-feu",
    onchain: "le coffre refuse on-chain",
    freeze: "gel d’un geste",
    bench: "banc : 200 robots, graine 42",
    end: "200 / 200 — F1 1,00",
  };

  /* ---------- easing / utilitaires ---------- */
  DR.ease = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
  DR.out = (x) => (x <= 0 ? 0 : x >= 1 ? 1 : 1 - Math.pow(1 - x, 3));
  DR.clamp = (x, a = 0, b = 1) => Math.max(a, Math.min(b, x));
  DR.lerp = (a, b, x) => a + (b - a) * x;
  DR.mix3 = (a, b, x) => [DR.lerp(a[0], b[0], x), DR.lerp(a[1], b[1], x), DR.lerp(a[2], b[2], x)];

  /* ---------- construction des beats ---------- */
  function buildBeats(scenarios, opts) {
    const bench = !opts || opts.bench !== false;
    const beats = [];
    const push = (sc, ph, dur, extra) => beats.push(Object.assign({ sc, ph, dur, pass: 0, n: 0 }, extra || {}));

    scenarios.forEach((S, i) => {
      if (S.freeze) { push(i, "freeze", 3.2); return; }
      push(i, "arrive", 2.2);
      if (S.bypass) { push(i, "bypass", 2.0); push(i, "onchain", 2.6); return; }
      push(i, "send", 1.4);
      S.passes.forEach((P, pi) => {
        const fast = pi > 0 || i > 0;
        push(i, "sandbox", fast ? 3.0 : 4.2, { pass: pi, n: 5 });
        push(i, "effects", fast ? 1.3 : 1.8, { pass: pi });
        push(i, "rules", fast ? 1.3 : 1.8, { pass: pi });
        push(i, "verdict", 1.4, { pass: pi });
        if (P.verdict === "WARN") push(i, "approve", 2.0, { pass: pi });
        if (P.verdict === "ALLOW") {
          push(i, "cert", 2.4, { pass: pi, n: 7 });
          push(i, "chain", 1.8, { pass: pi });
          push(i, "proof", 2.2, { pass: pi });
        }
        if (P.verdict === "BLOCK") {
          push(i, "blocked", 1.8, { pass: pi });
          if (S.mcp) push(i, "mcp", 2.6, { pass: pi });
        }
      });
    });
    if (bench) { push(-1, "bench", 5.5); push(-1, "end", 1.0); }

    let acc = 0;
    beats.forEach((b) => { b.t0 = acc; acc += b.dur; b.t1 = acc; });
    return { beats, T: acc };
  }

  /* Change la liste des scénarios joués (démo scriptée ou réponse réelle). */
  DR.setScenarios = function (list, opts) {
    DR.scenarios = list;
    const built = buildBeats(list, opts);
    DR.beats = built.beats;
    DR.T = built.T;
    if (DR.app && DR.app.onScenarios) DR.app.onScenarios();
  };

  /* ---------- état complet à l'instant t ---------- */

  /* Accès temporels relatifs au scénario courant. */
  function attachHelpers(s, scNow, bi, t) {
    s.since = (ph, pass) => {
      const x = DR.beats.find((y, k) => y.sc === scNow && y.ph === ph && (pass == null || y.pass === pass) && k <= bi);
      return x ? t - x.t0 : -1;
    };
    s.reached = (ph, pass) => s.since(ph, pass) >= 0;
    s.k = (ph, pass) => {
      const x = DR.beats.find((y, k) => y.sc === scNow && y.ph === ph && (pass == null || y.pass === pass) && k <= bi);
      return x ? DR.clamp((t - x.t0) / x.dur) : 0;
    };
  }

  /* Effets, règles et verdict du passage courant. */
  function passState(s, S, lastPass) {
    const P = S && S.passes.length ? S.passes[lastPass] : null;
    if (!P) return null;
    if (s.reached("effects", lastPass)) s.effects = P.effects;
    if (s.reached("rules", lastPass)) {
      const rk = s.k("rules", lastPass);
      s.rules = P.rules.filter((id) => rk > (parseInt(id.slice(1), 10) - 0.5) / 12);
      s.rulesScan = rk;
    }
    if (s.reached("verdict", lastPass)) s.verdict = P.verdict;
    return P;
  }

  /* Résultats cumulés : historique, dépense 24 h, gel, journal, banc. */
  function cumulative(s, t, scNow) {
    s.history = [];
    let spent = 0n;
    DR.scenarios.forEach((X, i) => {
      const endB = DR.beats.filter((y) => y.sc === i).pop();
      if (endB && t >= endB.t1 - 0.001 && i !== scNow) {
        s.history.push({ i, id: X.id, end: X.end });
        if (X.end === "executed") {
          const last = X.passes[X.passes.length - 1];
          spent += last && last.effects[0] ? -last.effects[0].d : 0n;
        }
      }
    });
    const S = DR.scenarios[scNow];
    if (S && S.end === "executed" && s.reached("chain")) spent += -S.passes[S.passes.length - 1].effects[0].d;
    s.spent = spent;
    s.frozen = DR.scenarios.some((X, i) => X.freeze && (i < scNow || (i === scNow && s.k("freeze") > 0.45)));
    s.journal = DR.scenarios.reduce((n, X, i) => n + (X.passes.length
      ? (i < scNow ? X.passes.length : i === scNow ? (s.reached("verdict", s.lastPass) ? s.lastPass + 1 : s.lastPass) : 0)
      : 0), 0);
    s.bench = s.ph === "bench" ? DR.ease(s.p) : s.ph === "end" ? 1 : 0;
  }

  /* Légende courte de l'étape en cours. */
  function captionOf(s, S, P) {
    let cap = DR.PH[s.ph] || "";
    if (s.ph === "sandbox") cap = (s.lastPass ? "re‑simulation · " : "bac à sable · ") + (s.sub + 1) + "/5 · " + DR.sandboxSteps[s.sub].k;
    if (s.ph === "cert") cap = "certificat · " + (s.sub + 1) + "/7 · " + DR.certChecks[s.sub].k;
    if (s.ph === "verdict" && P) cap = "verdict : " + P.verdict + (P.rules.length ? " · " + P.rules.join(" ") : "");
    if (s.ph === "approve" && S && S.approveLabel) cap = S.approveLabel;
    if (s.ph === "proof" && S && S.proofLabel) cap = S.proofLabel;
    if (s.ph === "onchain") cap = "le coffre refuse on-chain · " + S.onchainErr;
    if (s.ph === "arrive" && S) cap = S.tag + " · " + S.say;
    return cap;
  }

  DR.at = function (t) {
    t = DR.clamp(t, 0, DR.T);
    let bi = DR.beats.findIndex((b) => t < b.t1);
    if (bi < 0) bi = DR.beats.length - 1;
    const b = DR.beats[bi];
    const p = DR.clamp((t - b.t0) / b.dur);
    const s = {
      t, T: DR.T, g: DR.T ? t / DR.T : 0, bi, beat: b, ph: b.ph, p, pass: b.pass,
      scI: b.sc, sc: b.sc >= 0 ? DR.scenarios[b.sc] : null,
      sub: b.n ? Math.min(b.n - 1, Math.floor(p * b.n)) : 0,
      subP: b.n ? DR.clamp(p * b.n - Math.floor(p * b.n)) : 0,
      caption: "", verdict: null, rules: [], effects: [],
    };
    const scNow = b.sc >= 0 ? b.sc : DR.scenarios.length - 1;
    attachHelpers(s, scNow, bi, t);
    s.lastPass = DR.beats.filter((x, k) => x.sc === scNow && k <= bi).reduce((m, x) => Math.max(m, x.pass), 0);
    s.scView = DR.scenarios[scNow] || DR.scenarios[0] || { passes: [], to: null };
    s.scViewI = scNow;
    const P = passState(s, DR.scenarios[scNow], s.lastPass);
    cumulative(s, t, scNow);
    s.caption = captionOf(s, DR.scenarios[scNow], P);
    return s;
  };

  /* premier montage : démo scriptée complète (banc inclus) */
  DR.setScenarios(DR.SCRIPTED, { bench: true });
})();
