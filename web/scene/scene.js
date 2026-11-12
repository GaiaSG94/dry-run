/* dry-run · scène « Plateau » (design V9) — même récit, mêmes données que web/scene,
   mais chaque élément a SA place, donnée par un agencement (LY) :
   aucun volume n'en traverse un autre, et tous les textes passent par le
   calque d'étiquettes (labels9.js) : à plat, sur plaque, sans chevauchement.
   Chaque étape porte un numéro (1 demande → 7 chaîne) : on lit la scène
   comme un schéma, de gauche à droite (ou en serpentin sur le plateau).
   Usage : DR.scene9(THEME, LAYOUT). */
(function () {
  "use strict";
  const E = DR.ease, O = DR.out, C = DR.clamp, L = DR.lerp, M = DR.mix3, V = () => DR.v;

  // étape (numéro) en cours selon la phase
  const STEP_OF = { arrive: 1, send: 2, sandbox: 3, effects: 3, rules: 4, verdict: 5, approve: 5, blocked: 5, mcp: 5, cert: 6, chain: 7, proof: 7, bypass: 6, onchain: 6, freeze: 6 };

  DR.scene9 = function (T, LY) {
    const P = LY.P, ink = "#ffffff", AC = T.accent;
    const mirror = !!LY.mirror;
    const twinV = LY.twinV, twinR = LY.twinR;   // copies (bac à sable)
    const ST = LY.stations;                     // 5 stations du bac à sable
    const SAND_IN = LY.sandIn;                   // entrée du bac à sable
    const VIA = Object.assign({ send: [], rules: [], cert: [] }, LY.via || {}); // détours : la demande contourne les volumes

    /* ---------- outils ---------- */
    const active = (s, n) => s.mode !== "idle" && STEP_OF[s.ph] === n;
    const lit = (s, n) => (active(s, n) ? 1 : s.mode === "idle" ? 0.85 : LY.focus ? 0.38 : 0.75); // les étapes hors-jeu s'effacent
    const tw = (s) => { const k = s.since("sandbox"); return k < 0 ? 0.35 : 0.35 + 0.65 * C(k / 0.7); };
    function step(R, s, n, at, name, sub, side, top) {
      const on = active(s, n);
      if (LY.stepSide === "above" && top) { at = top; side = "above"; }
      R.label(at, name, { num: n, sub, pr: on ? 1 : 4, size: on ? 15.5 : 13.5, plate: on ? "accent" : "dark", side: side || "below", alpha: s.mode === "idle" || on ? 1 : LY.focus ? 0.6 : 0.85 });
    }
    function plinth(R, c, r, on) { // socle rond posé au sol : chaque élément a son emplacement
      R.circle([c[0], 0.01, c[2]], r, "y", { fill: on ? R.A(0.1) : R.Wh(0.025), stroke: on ? R.A(0.9) : R.Wh(0.22), lw: on ? 2 : 1.2, zb: 1500 }, 56);
    }

    /* ---------- sol ---------- */
    function floor(R, s) {
      const F = LY.floor, th = 0.5;
      R.poly([[F[0], 0, F[2]], [F[1], 0, F[2]], [F[1], 0, F[3]], [F[0], 0, F[3]]], {
        zb: 2000, nofog: true,
        grad: (ctx, q) => { const g = ctx.createLinearGradient(0, (q[2].y + q[3].y) / 2, 0, (q[0].y + q[1].y) / 2); g.addColorStop(0, R.A(0.1 * (T.sheen || 1))); g.addColorStop(1, R.A(0.012)); return g; },
      });
      if (!mirror) R.poly([[F[0], 0, F[3]], [F[1], 0, F[3]], [F[1], -th, F[3]], [F[0], -th, F[3]]], { fill: R.A(0.1), stroke: R.A(0.35), lw: 1, zb: 1990 });
      R.line([F[0], 0, F[3]], [F[1], 0, F[3]], { stroke: R.A(0.6), lw: 1.6, glow: 8, zb: 1989 });
      // chemin au sol : la route que suit la demande (plateau)
      if (LY.floorPath) {
        const W = LY.floorPath;
        for (let i = 1; i < W.length; i++) R.line([W[i - 1][0], 0.02, W[i - 1][2]], [W[i][0], 0.02, W[i][2]], { stroke: R.A(0.32), lw: 6, zb: 1980 });
        for (let i = 1; i < W.length; i++) R.line([W[i - 1][0], 0.03, W[i - 1][2]], [W[i][0], 0.03, W[i][2]], { stroke: R.A(0.8), lw: 1.4, dash: [4, 7], zb: 1979 });
      }
      if (s.verdict === "BLOCK") { const k = O(C(s.since("verdict") / 0.6)); R.circle([P.rules[0], 0.02, P.rules[2]], 0.5 + 3 * k, "y", { stroke: R.Wh(1 - k), lw: 3, zb: -3 }, 64); }
      if (s.verdict === "ALLOW") { const k = O(C(s.since("verdict", s.lastPass) / 0.8)); R.circle([P.rules[0], 0.02, P.rules[2]], 0.5 + 3 * k, "y", { stroke: R.A(1 - k), lw: 3, zb: -3 }, 64); }
    }

    /* ---------- 1 · agent + sa demande ---------- */
    function agent(R, s) {
      const S = s.scView, c = P.agent, on = active(s, 1), dim = S.bypass ? 0.3 : 1, sp = s.clock * 0.8;
      plinth(R, c, 1.1, on);
      R.halo(c, 1.3, AC, (0.22 + (on ? 0.25 : 0)) * dim);
      const p1 = R.ring(c, 0.7, "y", 48).map((v) => [v[0], c[1] + (v[2] - c[2]) * Math.sin(sp), c[2] + (v[2] - c[2]) * Math.cos(sp)]);
      R.path(p1, { stroke: R.A(0.95 * dim), lw: 2.2 });
      R.circle(c, 0.7, "z", { stroke: R.A(0.6 * dim), lw: 1.6 }, 48);
      R.dot(c, 0.16, { fill: R.A(dim), glow: 14 });
      R.line([c[0], c[1] - 0.8, c[2]], [c[0], 0.02, c[2]], { stroke: R.A(0.3 * dim), dash: [3, 5] });
      const m = S.motif;
      step(R, s, 1, [c[0], 0, c[2] + 1.1], S.bypass ? "agent · clé volée" : "agent IA", m ? m.deposit + " → " + (S.to ? S.to.short : m.receiver) : null, null, [c[0], c[1] + 1.0, c[2]]);
    }

    /* ---------- 2 · pare-feu (portique) ---------- */
    function gate(R, s) {
      const c = P.gate, on = active(s, 2), h = LY.gateH || 4.2, d = LY.gateD || 2.8, w = 0.45;
      // portique : deux montants + traverse, une membrane de verre au milieu
      [-1, 1].forEach((sg) => R.box([c[0], h / 2, c[2] + sg * (d / 2)], [w, h, w], { shade: true, fa: 0.25, tint: AC, stroke: R.A(on ? 1 : 0.7), lw: on ? 2.2 : 1.5 }));
      R.box([c[0], h + w / 2, c[2]], [w, w, d + w], { shade: true, fa: 0.25, tint: AC, stroke: R.A(on ? 1 : 0.7), lw: on ? 2.2 : 1.5 });
      R.poly([[c[0], 0.05, c[2] - d / 2], [c[0], h, c[2] - d / 2], [c[0], h, c[2] + d / 2], [c[0], 0.05, c[2] + d / 2]], { fill: R.A(on ? 0.16 : 0.07), stroke: null });
      const sw = (s.clock * 0.5) % 1;
      R.line([c[0], 0.1 + sw * (h - 0.2), c[2] - d / 2], [c[0], 0.1 + sw * (h - 0.2), c[2] + d / 2], { stroke: R.A(0.45), lw: 1.5, glow: 8 });
      step(R, s, 2, [c[0], 0, c[2] + d / 2 + 0.5], "pare-feu", "CLI · HTTP · MCP", null, [c[0], h + 0.6, c[2]]);
    }

    /* ---------- 3 · bac à sable ---------- */
    function vaultShape(R, c, sz, o) { R.box(c, sz, Object.assign({ shade: true }, o)); }
    function recipShape(R, c, kind, k, o, sg) {
      if (kind === "dex") R.prism(c, 1.0 * k, 2.3 * k, 6, o, 0.5);
      else if (kind === "attacker") {
        const ap = [c[0], c[1] + sg * 1.25 * k, c[2]], b = c[1] - sg * 1.1 * k;
        const B = [0, 1, 2, 3].map((i) => { const an = 0.4 + (i / 4) * Math.PI * 2; return [c[0] + Math.cos(an) * 1.0 * k, b, c[2] + Math.sin(an) * 1.0 * k]; });
        for (let i = 0; i < 4; i++) R.poly([ap, B[i], B[(i + 1) % 4]], { fill: R.Wh(0.06), stroke: o.stroke, lw: o.lw });
      } else { R.box(c, [2.0 * k, 2.3 * k, 2.0 * k], Object.assign({ shade: true }, o)); R.box(c, [0.8 * k, 1.0 * k, 0.8 * k], { fill: "rgba(0,0,0,.75)", stroke: o.stroke }); }
    }
    function sandbox(R, s) {
      const S = s.scView, on = active(s, 3), a = tw(s), inSand = s.ph === "sandbox";
      if (!mirror) { // une boîte de verre en pointillés = « une copie de la chaîne »
        const c = P.sand, w = LY.sandW, d = LY.sandD, h = LY.sandH || 3.4;
        const pts = [[-1, -1], [1, -1], [1, 1], [-1, 1]];
        [0, h].forEach((y) => R.poly(pts.map(([u, v]) => [c[0] + (u * w) / 2, y + 0.02, c[2] + (v * d) / 2]), { stroke: R.A(on ? 0.95 : 0.5), lw: on ? 2 : 1.3, dash: [7, 6], fill: y === 0 ? R.A(on ? 0.08 : 0.035) : null, zb: y === 0 ? 1400 : 0 }));
        pts.forEach(([u, v]) => R.line([c[0] + (u * w) / 2, 0, c[2] + (v * d) / 2], [c[0] + (u * w) / 2, h, c[2] + (v * d) / 2], { stroke: R.A(on ? 0.85 : 0.4), lw: on ? 1.8 : 1.2, dash: [7, 6] }));
        step(R, s, 3, [c[0], 0, c[2] + d / 2 + 0.3], "bac à sable", "copie de la chaîne · hors chaîne");
      } else {
        step(R, s, 3, LY.sandLabelAt, "bac à sable", "le reflet : une copie de la chaîne");
      }
      // copies du coffre et du destinataire
      const sz = LY.twinSize || [1.3, 2.2, 1.3];
      vaultShape(R, twinV, sz, { fa: 0.12 * a, tint: AC, stroke: R.A(0.75 * a), lw: 1.5, alpha: Math.max(0.4, a) });
      R.label([twinV[0], twinV[1] + (mirror ? -1 : 1) * (sz[1] / 2 + 0.3), twinV[2]], "copie du coffre", { pr: 6, size: 11.5, weight: 600, plate: "none", color: R.A(0.85 * a), side: mirror ? "below" : "above", gap: 4 });
      if (S.to) {
        recipShape(R, twinR, S.to.kind, LY.twinK || 0.75, { fa: 0.1 * a, tint: AC, stroke: R.A(0.75 * a), lw: 1.5, alpha: Math.max(0.4, a) }, mirror ? -1 : 1);
        R.label([twinR[0], twinR[1] + (mirror ? -1.4 : 1.4), twinR[2]], "copie · " + S.to.short, { pr: 6, size: 11.5, weight: 600, plate: "none", color: R.A(0.85 * a), side: mirror ? "below" : "above", gap: 4 });
      }
      // 5 stations
      ST.forEach((p, i) => {
        const done = s.reached("sandbox") && (!inSand || s.sub > i || (s.sub === i && s.subP > 0.35)), cur = inSand && s.sub === i;
        if (LY.stationTiles) R.poly(R.ring([p[0], p[1] + 0.03, p[2]], 0.34, "y", 4, Math.PI / 4, Math.PI / 4 + Math.PI * 2).slice(0, 4), { fill: done ? R.A(0.6) : R.Wh(0.05), stroke: cur ? R.A(1) : done ? R.A(0.9) : R.A(0.4), lw: cur ? 2.4 : 1.3, glow: cur ? 14 : 0, zb: 1300 });
        else R.circle(p, 0.42, "x", { fill: done ? R.A(0.35) : R.Wh(0.03), stroke: cur ? R.A(1) : done ? R.A(0.9) : R.A(0.4), lw: cur ? 2.6 : 1.4, glow: cur ? 14 : 0 }, 36);
        if (cur) R.label(p, (i + 1) + "/5 · " + DR.sandboxSteps[i].k, { pr: 1, size: 13, plate: "accent", side: mirror ? "below" : "below", gap: 12 });
      });
      for (let i = 1; i < ST.length; i++) R.line(ST[i - 1], ST[i], { stroke: R.A(0.3), dash: [2, 4] });
    }

    /* ---------- flux d'effets (copie, puis réel) ---------- */
    function flows(R, s) {
      const S = s.scView, Pp = S.passes.length ? S.passes[s.lastPass] : null;
      if (!Pp || !S.to) return;
      const out = Pp.effects.find((e) => e.acc === "vault" && e.d < 0n), inn = Pp.effects.find((e) => e.acc === "vault" && e.u !== "NEAR");
      const arc = (A0, B0, up, zo, k, col, zero, label, lp) => {
        const pts = [], n = 40;
        for (let i = 0; i <= Math.round(n * k); i++) { const u = i / n, p = M(A0, B0, u); p[1] += up * Math.sin(u * Math.PI); p[2] += zo; pts.push(p); }
        if (pts.length < 2) return;
        R.path(pts, { stroke: zero ? R.Wh(0.7) : col, lw: 3, dash: zero ? [5, 6] : null, glow: zero ? 0 : 10 });
        if (k >= 1 && !zero) for (let j = 0; j < 2; j++) { const u = (s.clock * 0.4 + j / 2) % 1, p = M(A0, B0, u); p[1] += up * Math.sin(u * Math.PI); p[2] += zo; R.dot(p, 0.09, { fill: R.Wh(1), glow: 10 }); }
        if (k > 0.6) R.label(pts[Math.floor(pts.length * lp)], label, { pr: 2, size: 15, mono: true, weight: 800, plate: "dark", color: zero ? "#fff" : R.A(1), side: up > 0 ? "above" : "below", gap: 6 });
      };
      const sg = mirror ? -1 : 1, hV = (LY.twinSize || [1.3, 2.2])[1] / 2;
      if (s.reached("effects", s.lastPass) && !(s.reached("chain") && s.since("chain") > 0.6)) {
        const k = O(C(s.since("effects", s.lastPass) / 1.1)), a = mirror ? -0.6 : 0.6;
        const A0 = [twinV[0] + 0.7, twinV[1] + sg * (hV - 0.3), twinV[2]], B0 = [twinR[0] - 0.6, twinR[1] + sg * 0.6, twinR[2]];
        if (out) arc(A0, B0, sg * (LY.arcUp || 1.4), -0.25, k, R.A(1), false, "− " + DR.fmt(-out.d, 2) + " NEAR", 0.5);
        if (inn) arc(B0, A0, sg * ((LY.arcUp || 1.4) * 0.55), 0.35, k, R.A(1), inn.d === 0n, (inn.d === 0n ? "0 " : "+ " + DR.fmt(inn.d, 2) + " ") + inn.u, 0.5);
        void a;
      }
      if (s.reached("chain")) {
        const k = O(C(s.since("chain") / 1.4));
        const A0 = [P.vault[0] + 0.8, P.vault[1] + 1.4, P.vault[2]], B0 = [P.recip[0] - 0.8, P.recip[1] + 1.1, P.recip[2]];
        if (out) arc(A0, B0, LY.realArcUp || 1.8, -0.2, k, R.A(1), false, "− " + DR.fmt(-out.d, 2) + " NEAR", 0.35);
        if (inn && inn.d > 0n) arc(B0, A0, (LY.realArcUp || 1.8) * 0.6, 0.3, k, R.A(1), false, "+ " + DR.fmt(inn.d, 2) + " " + inn.u, 0.4);
      }
    }

    /* ---------- 4 · règles ---------- */
    function rules(R, s) {
      const c = P.rules, on = active(s, 4) || active(s, 5), S = s.scView, scanning = s.ph === "rules" ? s.p : -1;
      const cols = LY.ruleCols || 12, gap = LY.ruleGap || 0.48, rows = Math.ceil(12 / cols);
      let hot = null;
      DR.rules.forEach((r, i) => {
        const col = i % cols, row = Math.floor(i / cols);
        const x = c[0] + (col - (cols - 1) / 2) * gap, z = c[2] + (row - (rows - 1) / 2) * gap * 1.4;
        const bit = s.rules.includes(r.id), ok5 = r.id === "R5" && S.passes.length > 1 && s.reached("approve");
        const near = scanning >= 0 && Math.abs(scanning * 1.08 - (i + 0.5) / 12) < 0.05;
        const ht = bit && !ok5, hg = ht ? 0.4 + 1.3 * O(C(s.since("rules", s.lastPass) / 0.9 + 0.15)) : near ? 0.75 : 0.3;
        R.box([x, hg / 2, z], [0.34, hg, 0.34], { shade: true, fa: ht ? 0.7 : near ? 0.5 : 0.2, tint: ht ? ink : AC, stroke: ht ? R.Wh(1) : near ? R.A(1) : R.A(0.55), lw: 1.4 });
        if (ht) hot = { p: [x, hg + 0.2, z], r };
      });
      if (scanning >= 0) { const x = c[0] + (C(scanning * 1.08) * cols - cols / 2) * gap; R.line([x, 0.03, c[2] - rows * gap], [x, 0.03, c[2] + rows * gap], { stroke: R.A(1), lw: 2.5, glow: 16 }); }
      if (hot) R.label(hot.p, hot.r.id + " · " + hot.r.k, { pr: 1, size: 15, plate: "white", side: "above", gap: 10 });
      step(R, s, 4, [c[0], 0, c[2] + rows * gap * 0.8 + 0.4], "règles", "12 contrôles sur les effets", null, [c[0], 1.2, c[2]]);
      void on;
    }

    /* ---------- 5 · verdict ---------- */
    function verdict(R, s) {
      const c = P.verdict;
      if (s.mode === "idle" && LY.verdictSlot) step(R, s, 5, LY.verdictSlot, "verdict", "ALLOW · WARN · BLOCK", "above");
      if (!s.verdict) return;
      const k = O(C(s.since("verdict", s.lastPass) / 0.5)), v = s.verdict;
      const fade = s.reached("chain") ? 1 - C(s.since("chain") / 0.6) : s.ph === "approve" ? 1 - s.p * 0.5 : 1;
      if (fade <= 0.02) return;
      const o = { pr: 0, size: L(30, LY.verdictPx || 54, k), weight: 300, ls: 10, plate: "none", side: "center", alpha: fade, fixed: true, noLead: true };
      if (v === "ALLOW") R.label(c, "ALLOW", { ...o, color: R.A(1), glow: 30 });
      if (v === "WARN") R.label(c, "WARN", { ...o, color: "#fff", outline: 1.6 });
      if (v === "BLOCK") R.label(c, "BLOCK", { ...o, color: "#fff", weight: 500, glow: 16, glowColor: "rgba(255,255,255,.6)" });
    }

    /* ---------- 6 · coffre + certificat ---------- */
    function vault(R, s) {
      const VN = DR.vaultNow(s), c = P.vault, fr = VN.frozen, on = active(s, 6), sz = LY.vaultSize || [1.8, 3.4, 1.8];
      plinth(R, c, 1.9, on);
      R.pool([c[0], 0.01, c[2]], 2.4, fr ? ink : AC, 0.18);
      vaultShape(R, [c[0], sz[1] / 2, c[2]], sz, { fa: fr ? 0.3 : T.glassA || 0.09, tint: ink, stroke: fr ? R.Wh(1) : R.Wh(0.85), lw: fr ? 2.6 : 1.8 });
      const ratio = Number((VN.balance * 1000n) / DR.vault.balance) / 1000, hh = (sz[1] - 0.2) * C(ratio);
      R.box([c[0], 0.1 + hh / 2, c[2]], [sz[0] - 0.3, Math.max(0.02, hh), sz[2] - 0.3], { shade: true, fa: 0.16, tint: AC, stroke: R.A(0.4) });
      // fenêtre 24 h : 24 segments au sol autour du socle
      const used = VN.dailyCap > 0n ? Math.round(Number((VN.spent * 24n) / VN.dailyCap)) : 0;
      for (let i = 0; i < 24; i++) { const a0 = (i / 24) * Math.PI * 2, a1 = a0 + ((Math.PI * 2) / 24) * 0.7; R.path(R.ring([c[0], 0.03, c[2]], 1.6, "y", 6, a0, a1), { stroke: i < used ? R.A(1) : R.Wh(0.25), lw: i < used ? 3 : 1.6, zb: 1200 }); }
      step(R, s, 6, [c[0], 0, c[2] + 2.0], "coffre", DR.fmt(VN.balance, 0) + " NEAR · max 2 / tx · 10 / 24 h", null, [c[0], sz[1] + 0.3, c[2]]);
      if (fr) R.label([c[0], sz[1] * 0.6, c[2]], "FROZEN", { pr: 0, size: 26, weight: 900, ls: 4, plate: "white", side: "center", fixed: true });
      // certificat : un sceau qui se pose au-dessus du coffre
      if (s.reached("cert") && !(s.reached("chain") && s.since("chain") > 0.5)) {
        const inC = s.ph === "cert", n = inC ? s.sub + (s.subP > 0.5 ? 1 : 0) : 7, sc = P.cert, rot = s.clock * 0.6;
        const ck = inC ? C(s.p * 2.5) : 1, cc = M([sc[0], sc[1] + 1.2, sc[2]], sc, O(ck));
        for (let i = 0; i < 7; i++) { const a0 = rot + (i / 7) * Math.PI * 2 + 0.06, a1 = a0 + (Math.PI * 2) / 7 - 0.14, on2 = i < n; R.path(R.ring(cc, 0.75, "z", 10, a0, a1), { stroke: on2 ? R.A(1) : R.Wh(0.3), lw: on2 ? 4 : 2, glow: on2 ? 10 : 0 }); }
        R.halo(cc, 1.2, AC, 0.25);
        R.label(cc, inC ? "certificat " + (s.sub + 1) + "/7 · " + DR.certChecks[s.sub].k : "certificat 7/7 ✓", { pr: 1, size: 13.5, plate: "accent", side: "right", gap: 30 });
      }
    }

    /* ---------- 7 · chaîne NEAR + destinataire ---------- */
    function chain(R, s) {
      const A0 = P.chainA, B0 = P.chainB, n = LY.chainN || 7, run = s.ph === "chain" ? E(C(s.p / 0.9)) : s.reached("chain") ? 1 : 0, on = active(s, 7);
      for (let i = 0; i < n; i++) {
        const u = i / (n - 1), p = M(A0, B0, u), lt = run > u;
        R.box([p[0], 0.28, p[2]], [0.5, 0.5, 0.5], { shade: true, fa: lt ? 0.6 : 0.12, tint: lt ? AC : ink, stroke: lt ? R.A(1) : R.Wh(0.4), lw: 1.4, glow: lt ? 8 : 0 });
        if (i) R.line([M(A0, B0, (i - 1) / (n - 1))[0] + 0.27, 0.28, M(A0, B0, (i - 1) / (n - 1))[2]], [p[0] - 0.27, 0.28, p[2]], { stroke: R.Wh(0.3), lw: 1.2 });
      }
      const mid = M(A0, B0, 0.5);
      const blk = s.since("blocked") >= 0 ? s.since("blocked") : s.since("onchain");
      step(R, s, 7, [mid[0], 0, mid[2] + 1.1], "chaîne NEAR", blk >= 0 ? "intacte : rien n’a été signé" : "exécution réelle", null, [mid[0], 0.8, mid[2]]);
      void on;
    }
    function recipient(R, s) {
      const S = s.scView;
      if (!S.to) return;
      const c = P.recip, got = s.reached("proof"), on = active(s, 7);
      plinth(R, c, 1.4, on || got);
      recipShape(R, [c[0], 1.35, c[2]], S.to.kind, 1, { fa: T.glassA || 0.09, tint: ink, stroke: R.Wh(0.8), lw: 1.7 }, 1);
      R.label(LY.stepSide === "above" ? [c[0], 3.0, c[2]] : [c[0], 0, c[2] + 1.5], S.to.short, { pr: 3, size: 15, sub: S.to.type, plate: S.to.kind === "dex" ? "dark" : "white", side: LY.stepSide === "above" ? "above" : "below" });
      if (got) R.label([c[0], 3.2, c[2]], "sand = réel ✓", { pr: 0, size: 22, weight: 900, plate: "accent", side: "above", gap: 6, alpha: C(s.since("proof") / 0.4) });
    }

    /* ---------- acteurs, boucle MCP ---------- */
    function actor(R, at, label, shape, target, solid) {
      R.halo(at, 1.1, ink, 0.16);
      if (shape === "owner") { R.dot(at, 0.26, { fill: R.Wh(0.95), glow: 12 }); R.circle([at[0], at[1] - 0.65, at[2]], 0.42, "z", { stroke: R.Wh(0.9), lw: 2 }, 20, Math.PI, Math.PI * 2); }
      if (shape === "thief") { const sp = [[at[0], at[1] + 0.7, at[2]], [at[0] - 0.5, at[1] - 0.5, at[2] - 0.35], [at[0] + 0.5, at[1] - 0.5, at[2] - 0.35], [at[0], at[1] - 0.5, at[2] + 0.5]]; [[0, 1, 2], [0, 2, 3], [0, 3, 1]].forEach((f) => R.poly(f.map((i) => sp[i]), { fill: R.Wh(0.12), stroke: R.Wh(0.95), lw: 1.6 })); }
      if (shape === "phone") R.box(at, [0.7, 1.25, 0.3], { shade: true, fa: 0.14, tint: ink, stroke: R.Wh(0.95), lw: 1.7 });
      R.label([at[0], at[1] + 0.9, at[2]], label, { pr: 2, size: 14, plate: "white", side: "above" });
      if (target) R.line(at, target.p, { stroke: R.Wh(0.9), lw: 2.4, dash: solid ? null : [7, 5], glow: solid ? 10 : 0 });
    }
    function actors(R, s) {
      const S = s.scView, vTop = [P.vault[0], (LY.vaultSize || [0, 3.4])[1] + 0.1, P.vault[2]];
      if (S.passes.length > 1 && s.reached("approve") && !(s.reached("sandbox", 1) && s.since("sandbox", 1) > 1.2)) {
        const k = s.ph === "approve" ? E(s.p) : 1; actor(R, LY.actorR, "propriétaire", "owner", { p: M(LY.actorR, P.verdict, k) });
      }
      if (S.bypass) {
        const kb = s.ph === "bypass" ? E(s.p) : s.reached("onchain") ? 1 : 0;
        actor(R, LY.actorV, "voleur", "thief", kb > 0 ? { p: M(LY.actorV, vTop, kb) } : null);
        if (s.reached("onchain")) R.label(vTop, S.onchainErr, { pr: 0, size: 16, mono: true, weight: 800, plate: "white", side: "above", gap: 14, sub: "refus on-chain par le coffre" });
      }
      if (S.freeze) { const k = s.k("freeze"); actor(R, LY.actorV, "téléphone", "phone", { p: M(LY.actorV, vTop, E(C(k * 2.2))) }, true); }
      if (S.mcp && s.reached("mcp")) {
        const k = s.ph === "mcp" ? s.p : 1, a = [P.verdict[0], P.verdict[1] - 0.5, P.verdict[2]], b = [P.agent[0], P.agent[1] + 0.9, P.agent[2]], pts = [];
        for (let i = 0; i <= 48; i++) { const u = i / 48, p = M(a, b, u); p[1] += Math.sin(u * Math.PI) * (LY.mcpUp || 3); pts.push(p); }
        R.path(pts, { stroke: R.Wh(0.85), lw: 2, dash: [7, 6] });
        R.halo(pts[Math.min(48, Math.floor(E(k) * 48))], 0.4, ink, 0.9);
        R.label(pts[24], "explication renvoyée à l’agent (MCP)", { pr: 2, size: 13.5, plate: "dark", side: "above" });
      }
    }

    /* ---------- la demande en vol ---------- */
    const way = (pts, u) => { // position le long d'une polyligne, u ∈ [0,1]
      const seg = []; let tot = 0;
      for (let i = 1; i < pts.length; i++) { const l = Math.hypot(...V().sub(pts[i], pts[i - 1])); seg.push(l); tot += l; }
      let d = C(u) * tot;
      for (let i = 0; i < seg.length; i++) { if (d <= seg[i] || i === seg.length - 1) return M(pts[i], pts[i + 1], seg[i] ? C(d / seg[i]) : 1); d -= seg[i]; }
      return pts[pts.length - 1];
    };
    function orbPos(s) {
      const S = s.scView;
      if (!S.passes.length) return null;
      const g = [P.gate[0], 1.6, P.gate[2]], rr = [P.rules[0], 1.0, P.rules[2]], ce = P.cert, va = [P.vault[0], 1.8, P.vault[2]];
      const ag = P.agent, ph = s.ph;
      if (ph === "arrive") return ag;
      if (ph === "send") return way([ag, g, ...VIA.send, SAND_IN], E(s.p));
      if (ph === "sandbox") return way([SAND_IN, ...ST], (s.sub + E(C(s.subP * 1.6))) / 5);
      if (ph === "effects") return ST[4];
      if (ph === "rules") return way([ST[4], ...VIA.rules, rr], E(C(s.p * 2)));
      if (["verdict", "approve"].includes(ph)) return rr;
      if (ph === "cert") return way([rr, ...VIA.cert, ce], E(C(s.p * 2.5)));
      if (ph === "chain") return way([ce, va, [P.chainA[0], 0.9, P.chainA[2]], [P.chainB[0], 0.9, P.chainB[2]], [P.recip[0], 1.4, P.recip[2]]], E(s.p));
      if (ph === "proof") return [P.recip[0], 1.4, P.recip[2]];
      return null;
    }
    const SHARDS = Array.from({ length: 30 }, (_, i) => [Math.sin(i * 12.9) * 0.5, Math.sin(i * 7.3) * 0.5, Math.sin(i * 3.1) * 0.5]);
    function orb(R, s) {
      if (s.ph === "blocked") { const k = O(C(s.p * 1.4)), c = [P.rules[0], 1.0, P.rules[2]]; SHARDS.forEach((d) => R.dot(V().add(c, V().mul(d, k * 3)), 0.06, { fill: R.Wh(1 - k) })); return; }
      const p = orbPos(s);
      if (!p) return;
      const al = s.ph === "proof" ? 1 - s.p : 1;
      for (let i = 0; i < 3; i++) { const r = ((s.clock * 0.9 + i / 3) % 1) * 0.8; R.circle(p, 0.2 + r, "z", { stroke: R.A(al * (1 - r / 0.8)), lw: 1.6 }, 32); }
      R.halo(p, 0.9, AC, 0.55 * al, { zb: -1 });
      R.dot(p, 0.14, { fill: R.Wh(al), zb: -1.1, glow: 20 });
    }

    /* ---------- banc ---------- */
    function bench(R, s) {
      if (s.ph !== "bench" && s.ph !== "end") return;
      const b = s.bench, cx = P.bench[0], cz = P.bench[2], sp = 0.36;
      const slot = (n, up) => [cx - 1.6 + (n % 10) * sp, 0.05, cz + (up ? -0.6 - Math.floor(n / 10) * sp : 0.6 + Math.floor(n / 10) * sp)];
      R.poly([[cx - 2, 0.02, cz - 4.4], [cx + 2, 0.02, cz - 4.4], [cx + 2, 0.02, cz - 0.3], [cx - 2, 0.02, cz - 0.3]], { fill: R.A(0.06), stroke: R.A(0.5), lw: 1.4, zb: 1000 });
      R.poly([[cx - 2, 0.02, cz + 0.3], [cx + 2, 0.02, cz + 0.3], [cx + 2, 0.02, cz + 4.4], [cx - 2, 0.02, cz + 4.4]], { fill: R.Wh(0.04), stroke: R.Wh(0.5), lw: 1.4, zb: 1000 });
      let up = 0, dn = 0;
      DR.benchRobots.forEach((rb, j) => {
        const idx = rb.atk ? dn++ : up++, k = C((b - (j / 200) * 0.78) / 0.18);
        if (k <= 0) return;
        const tg = slot(idx, !rb.atk), st = [cx, 5, cz];
        R.dot(M(st, tg, E(k)), 0.11, { fill: rb.atk ? R.Wh(0.95) : R.A(1), glow: k < 1 ? 8 : 0 });
      });
      const a = Math.max(0.4, C((b - 0.7) / 0.3));
      R.label([cx + 2.2, 0, cz - 2.3], "100 honnêtes", { pr: 1, size: 16, plate: "dark", color: R.A(1), sub: "acceptées · 0 faux refus", side: "right", alpha: a });
      R.label([cx + 2.2, 0, cz + 2.3], "100 attaques", { pr: 1, size: 16, plate: "white", sub: "bloquées · 0 passée", side: "right", alpha: a });
      R.label([cx, 0, cz - 4.6], "banc de test · 200 demandes", { pr: 0, size: 16, plate: "accent", side: "above" });
    }

    /* ---------- fond : mots du miroir (variante B) ---------- */
    function background(R) {
      const ctx = R.ctx, g = ctx.createRadialGradient(R.W * 0.5, -R.H * 0.15, 0, R.W * 0.5, -R.H * 0.15, R.H * 1.15);
      g.addColorStop(0, R.Wh(0.13)); g.addColorStop(0.5, R.A(0.04)); g.addColorStop(1, R.A(0)); ctx.fillStyle = g; ctx.fillRect(0, 0, R.W, R.H);
      if (!mirror) return;
      const a = R.p(-12, 0, 0), b = R.p(12, 0, 0);
      if (!a.ok || !b.ok) return;
      const size = C(R.W * 0.034, 22, 52), x = 14 + size * 0.8, y = a.y + ((b.y - a.y) * (x - a.x)) / (b.x - a.x || 1);
      ctx.save(); ctx.translate(x, C(y, R.H * 0.3, R.H * 0.7)); ctx.rotate(-Math.PI / 2);
      ctx.font = `200 ${size}px ${DR.SANS}`; ctx.letterSpacing = size * 0.3 + "px"; ctx.lineWidth = 1.4;
      ctx.textAlign = "left"; ctx.strokeStyle = R.Wh(0.9); ctx.strokeText("réel", 20, 0);
      ctx.textAlign = "right"; ctx.strokeStyle = R.A(1); ctx.strokeText("sand", -20, 0);
      ctx.restore();
    }

    const cam = LY.camera;
    DR.mount({
      id: LY.id, accent: AC, bg: T.bg, theme: T, nav: LY.nav, camera: cam, centerY: LY.centerY || 0.46,
      padLeft: (R) => (mirror ? 14 + C(R.W * 0.034, 22, 52) * 1.3 : 0),
      cameraAt(s) {
        if (s.ph === "bench" || s.ph === "end") return { target: P.bench, dist: LY.benchDist || 15, pitch: Math.max(cam.pitch, 0.55), yaw: 0 };
        if (s.mode === "idle" || !LY.follow) return {};
        const at = { 1: P.agent, 2: P.gate, 3: P.sand, 4: P.rules, 5: P.rules, 6: P.vault, 7: P.chainB }[STEP_OF[s.ph]];
        return at ? { target: M(cam.target, [at[0], cam.target[1], at[2]], LY.follow), dist: cam.dist * (LY.followZoom || 1) } : {};
      },
      background,
      draw(R, s) {
        floor(R, s); agent(R, s); gate(R, s); sandbox(R, s); flows(R, s); rules(R, s); verdict(R, s);
        vault(R, s); chain(R, s); recipient(R, s); actors(R, s); orb(R, s); bench(R, s);
      },
    });
  };
})();
