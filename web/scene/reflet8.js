/* dry-run · scène « LE REFLET » V7, pilotée par un thème (DR.reflet8(T)).
   Même récit et mêmes données que web/scene/reflet.js ; ajouts : volumes de verre ombrés,
   flaques de lumière, brume de profondeur, onde de choc du verdict, traînée de la demande,
   motifs de sol propres à chaque identité, particules d'ambiance, caméra chorégraphiée.
   Moins de texte, et l'essentiel en plus gros. */
(function () {
  "use strict";
  const E = DR.ease, O = DR.out, C = DR.clamp, L = DR.lerp, M = DR.mix3;
  const VA = [-8.6, 1.3, 0], VB = [-8.6, -1.3, 0], RA = [6.6, 1.3, 0], RB = [6.6, -1.3, 0];
  const GX = -3.2;
  const ST = [-1.6, -0.1, 1.4, 2.9, 4.4].map((x) => [x, -1.6, 0]);
  const RY = 1.65, RAIL = [-7.2, 5.3];
  const TK = [-5.45, 3.7, 1.0], TW = 3.0, TH = 2.0;
  const PORTY = [3.2, 2.65, 2.1];
  const PORT = [GX - 0.05, PORTY[2], 1.5], IN = [GX + 0.6, -1.6, 0];
  const SEAL = [GX + 1.5, 3.3, 0.8];
  const ACT = [-11.4, 4.6, 0.8];
  const BX = 12.6;
  const HIT = [VA[0] - 0.95, VA[1] + 1.2, 0.95];
  const VC = [0.6, 0.0, 0.6]; // centre du verdict sur le plan

  let sd = 7;
  const rnd = () => ((sd = (sd * 16807) % 2147483647) / 2147483647);
  const CRACKS = Array.from({ length: 14 }, () => {
    const a = rnd() * Math.PI * 2, pts = [[ST[4][0], 0.002, 0.4]];
    let x = ST[4][0], z = 0.4;
    const len = 1.6 + rnd() * 4.2;
    for (let k = 0; k < 8; k++) { const aa = a + (rnd() - 0.5) * 0.9; x += (Math.cos(aa) * len) / 8; z += ((Math.sin(aa) * len) / 8) * 0.6; pts.push([x, 0.002, z]); }
    return pts;
  });
  const SHARDS = Array.from({ length: 34 }, () => [rnd() - 0.5, rnd() - 0.5, rnd() - 0.5]);
  const PARTS = Array.from({ length: 140 }, () => [L(-15, 19, rnd()), L(-5, 7, rnd()), L(-9, 4, rnd()), rnd()]);

  DR.reflet8 = function (T) {
    const tw = (s) => {
      const k = s.since("sandbox");
      let a = k < 0 ? 0.22 : 0.22 + 0.78 * C(k / 0.7);
      if (s.reached("blocked")) a = L(a, 0.32, C(s.since("blocked") / 1.2));
      return a;
    };
    const appear = (s) => (s.scViewI === 0 ? 1 : C(Math.max(0, s.since("arrive")) / 0.6));
    const ink = T.ink || "#ffffff";
    const FA = T.wire ? 0 : T.glassA == null ? 0.06 : T.glassA;
    const VSZ = T.vault === "monolith" ? [1.35, 2.9, 1.35] : [1.9, 2.4, 1.9];

    /* ---------- fond : dégradé vertical + particules ---------- */
    function background(R, s) {
      const ctx = R.ctx;
      if (T.bgTop) { const g = ctx.createLinearGradient(0, 0, 0, R.H); g.addColorStop(0, T.bgTop); g.addColorStop(0.6, T.bg); g.addColorStop(1, T.bgLow || T.bg); ctx.fillStyle = g; ctx.fillRect(0, 0, R.W, R.H); }
      const hz = R.p(0.6, 0, -6);
      if (hz.ok) { const g = ctx.createRadialGradient(R.W / 2, hz.y, 0, R.W / 2, hz.y, R.W * 0.6); g.addColorStop(0, R.A(0.07 * (T.horizon == null ? 1 : T.horizon))); g.addColorStop(1, R.A(0)); ctx.fillStyle = g; ctx.fillRect(0, 0, R.W, R.H); }
      const c = s.clock, fx = T.bgFx;
      if (fx === "aurora") [[0.2, 0.3, 0.5], [0.78, 0.22, 0.45], [0.55, 0.72, 0.6], [0.08, 0.82, 0.4]].forEach(([bx, by, br], i) => {
        const x = R.W * (bx + 0.07 * Math.sin(c * 0.11 + i * 2)), y = R.H * (by + 0.06 * Math.cos(c * 0.09 + i)), rr = R.W * br * 0.62;
        const g = ctx.createRadialGradient(x, y, 0, x, y, rr); g.addColorStop(0, R.A(i % 2 ? 0.13 : 0.2)); g.addColorStop(1, R.A(0)); ctx.fillStyle = g; ctx.fillRect(0, 0, R.W, R.H);
      });
      if (fx === "spot") { const g = ctx.createRadialGradient(R.W * 0.5, -R.H * 0.15, 0, R.W * 0.5, -R.H * 0.15, R.H * 1.15); g.addColorStop(0, R.Wh(0.16)); g.addColorStop(0.5, R.A(0.05)); g.addColorStop(1, R.A(0)); ctx.fillStyle = g; ctx.fillRect(0, 0, R.W, R.H); }
      if (fx === "beams") for (let i = 0; i < 6; i++) {
        const x = R.W * (((i * 0.19 + c * 0.006 * (i % 2 ? 1 : -1)) % 1 + 1) % 1), w = R.W * (0.05 + 0.03 * (i % 3));
        const g = ctx.createLinearGradient(x - w, 0, x + w, 0); g.addColorStop(0, R.A(0)); g.addColorStop(0.5, R.A(0.07)); g.addColorStop(1, R.A(0));
        ctx.save(); ctx.transform(1, 0, -0.25, 1, R.H * 0.12, 0); ctx.fillStyle = g; ctx.fillRect(x - w, 0, w * 2, R.H); ctx.restore();
      }
      words(R, s); verdictHud(R, s);
    }
    /* « réel » en haut, « simulé » en bas : deux mots À PLAT (écran), jamais en perspective */
    function words(R, s) {
      const ctx = R.ctx, mode = T.words || "right", style = T.wordStyle || "solid";
      const a = R.p(-12, 0, 4), b = R.p(14, 0, 4);
      if (!a.ok || !b.ok) return;
      const yAt = (x) => a.y + ((b.y - a.y) * (x - a.x)) / (b.x - a.x || 1);
      const W1 = T.upper ? "RÉEL" : "réel", W2 = T.upper ? "SIMULÉ" : "simulé";
      const center = mode === "center";
      const size = center ? C(R.W * 0.15, 60, 280) : C(R.W * 0.06, 30, 100), x = center ? R.W / 2 : R.W - (R.narrow ? 16 : 36);
      const paint = (txt, px, py, col, colA) => {
        if (style === "outline") { ctx.strokeStyle = col; ctx.lineWidth = T.wordStroke || 1.6; ctx.strokeText(txt, px, py); }
        else { ctx.fillStyle = style === "ghost" ? colA : col; ctx.fillText(txt, px, py); }
      };
      ctx.save();
      if ((s.ph === "bench" || s.ph === "end") && !center) ctx.globalAlpha = 0.14; // le banc occupe la droite : les mots s'estompent
      else if (T.verdictHud && s.verdict) { // le verdict géant prend la place : les deux mots s'effacent
        const fade = s.reached("chain") ? 1 - C(s.since("chain") / 0.5) : s.ph === "approve" ? 1 - s.p * 0.7 : 1;
        ctx.globalAlpha = 1 - 0.9 * fade * C(s.since("verdict", s.lastPass) / 0.3);
      }
      ctx.font = `${T.wordWeight || 900} ${R.cond("display")}${size}px ${R.font("display")}`;
      ctx.letterSpacing = size * (T.wordLs == null ? 0.1 : T.wordLs) + "px";
      const c1 = R.Wh(0.92), c2 = R.A(1), g1 = R.Wh(T.ghostA || 0.1), g2 = R.A((T.ghostA || 0.1) * 1.5);
      if (mode === "rail") {
        const y = yAt(x - size * 0.5);
        ctx.translate(x - size * 0.1, y); ctx.rotate(-Math.PI / 2); ctx.textBaseline = "alphabetic";
        ctx.textAlign = "left"; paint(W1, 22, 0, c1, g1);
        ctx.textAlign = "right"; paint(W2, -22, 0, c2, g2);
      } else if (mode === "poles") {
        const lb = T.letterbox ? R.H * 0.09 : 0;
        ctx.textAlign = "right"; ctx.textBaseline = "top"; paint(W1, x, lb + R.H * 0.115, c1, g1);
        ctx.textBaseline = "alphabetic"; paint(W2, x, R.H * 0.8 - lb * 0.4, c2, g2);
      } else {
        const y = yAt(x), yMax = R.H * (T.frost ? 0.78 : 0.83) - size; // « simulé » ne descend jamais sur la légende
        ctx.textAlign = center ? "center" : "right";
        ctx.textBaseline = "alphabetic"; paint(W1, x, Math.min(y - size * 0.3, yMax - size * 0.5), c1, g1);
        ctx.textBaseline = "top"; paint(W2, x, Math.min(y + size * 0.3, yMax), c2, g2);
      }
      ctx.restore();
    }
    /* verdict géant à plat, derrière la scène (identités typographiques) */
    function verdictHud(R, s) {
      if (!T.verdictHud || !s.verdict) return;
      const ctx = R.ctx, k = O(C(s.since("verdict", s.lastPass) / 0.45)), v = s.verdict;
      const fade = s.reached("chain") ? 1 - C(s.since("chain") / 0.5) : s.ph === "approve" ? 1 - s.p * 0.7 : 1;
      const size = Math.min(R.W * 0.27, R.H * 0.42) * (0.8 + 0.2 * k);
      ctx.save(); ctx.globalAlpha = fade * k;
      ctx.font = `900 ${R.cond("display")}${size}px ${R.font("display")}`; ctx.textAlign = "center"; ctx.textBaseline = "middle"; ctx.letterSpacing = size * 0.04 + "px";
      const x = R.W / 2, y = R.H * 0.36;
      if (v === "ALLOW") { ctx.fillStyle = R.A(0.95); ctx.fillText(v, x, y); }
      if (v === "WARN") { ctx.strokeStyle = R.Wh(0.95); ctx.lineWidth = 3; ctx.setLineDash([18, 10]); ctx.strokeText(v, x, y); }
      if (v === "BLOCK") { ctx.fillStyle = R.Wh(0.96); ctx.fillText(v, x, y); ctx.fillStyle = R.BG; ctx.fillRect(x - size * 1.6, y - size * 0.045, size * 3.2, size * 0.07); }
      ctx.restore();
    }
    function particles(R, s) {
      const kind = T.particles || "dust", c = s.clock;
      if (kind === "none") return;
      PARTS.forEach(([x, y, z, r], i) => {
        let p;
        if (kind === "dust") p = [x + Math.sin(c * 0.2 + i) * 0.4, y + Math.cos(c * 0.17 + i) * 0.3, z];
        else if (kind === "spores") p = [x + Math.sin(c * 0.3 + i) * 0.3, ((y + 5 + c * (0.12 + r * 0.15)) % 12) - 5, z];
        else if (kind === "rain") { const yy = ((y + 5 - c * (2 + r * 2)) % 12 + 12) % 12 - 5; R.line([x, yy, z], [x, yy + 0.35, z], { stroke: R.A(0.18 + r * 0.2), lw: 1, zb: 30 }); return; }
        else if (kind === "stars") p = [x * 2, y * 1.6 + 2, -16 - r * 6];
        const tw2 = kind === "stars" ? 0.4 + 0.6 * Math.abs(Math.sin(c * (0.5 + r) + i)) : 1;
        R.dot(p, kind === "stars" ? 1.1 : 0.03 + r * 0.03, { px: kind === "stars", fill: i % 5 ? R.Wh(0.25 * tw2) : R.A(0.45 * tw2), zb: 30, nofog: kind === "stars" });
      });
    }

    /* ---------- le plan de verre ---------- */
    function plane(R, s) {
      const z0 = -14, z1 = 4, x0 = -14, x1 = 17;
      R.poly([[x0, 0, z0], [x1, 0, z0], [x1, 0, z1], [x0, 0, z1]], {
        zb: 2000, nofog: true,
        grad: (ctx, q) => { const g = ctx.createLinearGradient(0, (q[2].y + q[3].y) / 2, 0, (q[0].y + q[1].y) / 2); g.addColorStop(0, R.A(0.11 * (T.sheen || 1))); g.addColorStop(1, R.A(0.01)); return g; },
      });
      const th = 0.7; // dalle épaisse : tranche avant + deux flancs
      R.poly([[x0, 0, z1], [x1, 0, z1], [x1, -th, z1], [x0, -th, z1]], { fill: R.A(0.14), stroke: R.A(0.4), lw: 1 });
      [x0, x1].forEach((xx) => R.poly([[xx, 0, z0], [xx, 0, z1], [xx, -th, z1], [xx, -th, z0]], { fill: R.A(0.07), stroke: R.A(0.25), lw: 1 }));
      R.line([x0, -th, z1], [x1, -th, z1], { stroke: R.A(0.5), lw: 1 });
      const pat = T.plane || "grid", c = s.clock, P0 = [0.6, 0, 0];
      const hash = (n) => { const v = Math.sin(n * 12.9898) * 43758.5453; return v - Math.floor(v); };
      if (pat === "hex") {
        const r = 1.0, dx = r * 1.5, dz = r * Math.sqrt(3);
        let n = 0;
        for (let i = 0, x = x0 + 1; x <= x1 - 1; x += dx, i++) for (let z = z0 + 1 + (i % 2 ? dz / 2 : 0); z <= z1 - 0.9; z += dz) {
          n++;
          const hx = R.ring([x, 0, z], r * 0.93, "y", 6); hx.pop();
          const ph = Math.sin(c * 0.9 + n * 1.7), on = hash(n) < 0.14 && ph > 0;
          R.poly(hx, { stroke: R.A(0.2), fill: on ? R.A(0.22 * ph) : null, zb: 1999 });
        }
      }
      if (pat === "tiles") {
        const t = 1.5;
        let n = 0;
        for (let x = x0; x < x1 - 0.1; x += t) for (let z = z0; z < z1 - 0.1; z += t) {
          n++;
          const ph = Math.sin(c * 1.1 + n * 2.3), on = hash(n) < 0.1 && ph > 0;
          R.poly([[x, 0, z], [x + t, 0, z], [x + t, 0, z + t], [x, 0, z + t]], { stroke: R.Wh(0.1), lw: 1.2, fill: on ? R.A(0.5 * ph) : (Math.round(x / t) + Math.round(z / t)) % 2 ? R.Wh(0.022) : null, zb: 1999 });
        }
      }
      if (pat === "lines") {
        let n = 0;
        for (let z = z0 + 0.5; z < z1; z += 0.5) {
          n++;
          R.line([x0, 0, z], [x1, 0, z], { stroke: R.A(n % 4 === 0 ? 0.2 : 0.07), zb: 1999 });
          if (n % 3 === 0) { const u = (c * (0.12 + hash(n) * 0.2) + hash(n + 5)) % 1, xx = x0 + u * (x1 - x0 - 3); R.line([xx, 0, z], [xx + 2.4, 0, z], { stroke: R.A(0.85), lw: 2, glow: 10, zb: 1998 }); }
        }
      }
      if (pat === "contour") [[VA[0], 0, 0.9, 9], [RA[0], 0, 0.8, 9], [-1, -8, 1.0, 9], [12, -9, 0.9, 8]].forEach(([cx, cz, stp, cnt], ci) => {
        for (let i = 1; i <= cnt; i++) {
          const r = i * stp;
          let seg = [];
          const fl = () => { if (seg.length > 1) R.path(seg, { stroke: R.A(i % 3 === 0 ? 0.3 : 0.13), lw: i % 3 === 0 ? 1.3 : 1, zb: 1999 }); seg = []; };
          for (let k = 0; k <= 80; k++) { const a = (k / 80) * Math.PI * 2, rr = r * (1 + 0.1 * Math.sin(a * 3 + ci * 2 + i * 0.5) + 0.05 * Math.sin(a * 7 + i)), x = cx + Math.cos(a) * rr, z = cz + Math.sin(a) * rr; if (z > z1 || z < z0 || x < x0 || x > x1) fl(); else seg.push([x, 0, z]); }
          fl();
        }
      });
      if (pat === "grid" || pat === "neon") {
        const a = pat === "neon" ? R.A(0.16) : R.Wh(0.035);
        for (let x = -14; x <= 17; x += 1) R.line([x, 0, z0], [x, 0, z1], { stroke: a, zb: 1999 });
        for (let z = -13.5; z <= 3; z += 1.5) R.line([x0, 0, z], [x1, 0, z], { stroke: a, zb: 1999 });
        if (pat === "neon") { const sx = -14 + ((c * 5) % 31); R.line([sx, 0, z0], [sx, 0, z1], { stroke: R.A(0.8), lw: 2, glow: 16, zb: 1990 }); }
      }
      if (pat === "pinstripe") for (let x = -14; x <= 17; x += 0.5) R.line([x, 0, z0], [x, 0, z1], { stroke: R.Wh(x % 2 === 0 ? 0.06 : 0.025), zb: 1999 });
      if (pat === "dots") for (let x = -14; x <= 17; x += 1) for (let z = -13; z <= 3; z += 1) R.dot([x, 0, z], 0.035, { fill: R.Wh(0.22), zb: 1999 });
      if (pat === "rings") for (let i = 1; i < 26; i++) {
        const r = i * 0.75;
        let seg = [];
        const flush = () => { if (seg.length > 1) R.path(seg, { stroke: R.A(i % 4 === 0 ? 0.2 : 0.085), lw: i % 4 === 0 ? 1.4 : 1, zb: 1999 }); seg = []; };
        for (let k = 0; k <= 96; k++) { const a = (k / 96) * Math.PI * 2, rr = r * (1 + 0.035 * Math.sin(a * 5 + i * 1.7)), z = P0[2] + Math.sin(a) * rr, x = P0[0] + Math.cos(a) * rr; if (z > z1 || z < z0 || x < x0 || x > x1) flush(); else seg.push([x, 0, z]); }
        flush();
      }
      if (pat === "ripples") {
        for (let i = 0; i < 6; i++) { const r = (c * 0.7 + i * 1.6) % 9.6; R.circle(P0, r, "y", { stroke: R.A(0.22 * (1 - r / 9.6)), lw: 1.4, zb: 1999 }, 72); }
        const dv = s.since("sandbox");
        if (dv >= 0 && dv < 2.5) for (let i = 0; i < 3; i++) { const r = Math.max(0, dv * 2.2 - i * 0.6); R.circle([IN[0], 0, 0], r, "y", { stroke: R.A(0.6 * C(1 - r / 5)), lw: 2, zb: 1998 }, 60); }
      }
      if (pat === "radar") {
        [2, 4, 6, 8, 10].forEach((r) => R.circle(P0, r, "y", { stroke: R.A(0.1), zb: 1999 }, 90));
        for (let k = 0; k < 12; k++) { const a = (k / 12) * Math.PI * 2; R.line(P0, [P0[0] + Math.cos(a) * 10, 0, P0[2] + Math.sin(a) * 10], { stroke: R.A(0.06), zb: 1999 }); }
        const a0 = c * 0.9;
        for (let k = 0; k < 14; k++) { const a = a0 - k * 0.04; R.line(P0, [P0[0] + Math.cos(a) * 10, 0, P0[2] + Math.sin(a) * 10], { stroke: R.A(0.32 * (1 - k / 14)), lw: 2, zb: 1998 }); }
      }
      R.line([x0, 0, z1], [x1, 0, z1], { stroke: R.A(0.7), lw: 1.6, zb: 1997, glow: 8 });
      if (s.verdict === "BLOCK") {
        const k = O(C(s.since("verdict") / 0.9));
        CRACKS.forEach((cr) => { const n = Math.max(2, Math.round(cr.length * k)); R.path(cr.slice(0, n), { stroke: R.Wh(0.9), lw: 1.8, glow: 12, glowColor: R.Wh(0.7), zb: -5 }); });
      }
      // onde de choc du verdict
      if (s.verdict) {
        const k = s.since("verdict", s.lastPass);
        if (k >= 0 && k < 1.6) for (let i = 0; i < 2; i++) {
          const r = Math.max(0, O(C(k / 1.3)) * 7 - i * 0.9), al = C(1 - k / 1.4);
          R.circle(VC, r, "y", { stroke: s.verdict === "ALLOW" ? R.A(al) : R.Wh(al * 0.9), lw: 3 - i, dash: s.verdict === "WARN" ? [8, 7] : null, glow: 14, zb: -2 }, 80);
        }
      }
    }

    /* Arc ou anneau ÉPAIS : un arc extrudé le long de son axe (bande ombrée + deux bords).
       Sert aux anneaux du bac à sable, au sceau du certificat et à la fenêtre 24 h. */
    function thick(R, c, r, axis, a0, a1, depth, o) {
      const n = Math.max(3, Math.round(((a1 - a0) / (Math.PI * 2)) * (o.seg || 28))), A = [], B = [], h = depth / 2;
      const col = o.tint || T.accent, fa = T.wire ? 0 : o.fa == null ? 0.3 : o.fa;
      for (let i = 0; i <= n; i++) {
        const a = a0 + ((a1 - a0) * i) / n, ca = Math.cos(a) * r, sa = Math.sin(a) * r;
        if (axis === "x") { A.push([c[0] - h, c[1] + ca, c[2] + sa]); B.push([c[0] + h, c[1] + ca, c[2] + sa]); }
        else if (axis === "y") { A.push([c[0] + ca, c[1] - h, c[2] + sa]); B.push([c[0] + ca, c[1] + h, c[2] + sa]); }
        else { A.push([c[0] + ca, c[1] + sa, c[2] - h]); B.push([c[0] + ca, c[1] + sa, c[2] + h]); }
      }
      if (fa > 0) for (let i = 0; i < n; i++) {
        const a = a0 + ((a1 - a0) * (i + 0.5)) / n;
        const nr = axis === "x" ? [0, Math.cos(a), Math.sin(a)] : axis === "y" ? [Math.cos(a), 0, Math.sin(a)] : [Math.cos(a), Math.sin(a), 0];
        const li = 0.25 + 0.75 * Math.abs(DR.v.dot(nr, R.LIGHT));
        R.poly([A[i], A[i + 1], B[i + 1], B[i]], { fill: DR.rgba(col, fa * li), alpha: o.alpha });
      }
      const closed = a1 - a0 >= Math.PI * 2 - 1e-6;
      [A, B].forEach((P) => R.poly(closed ? P.slice(0, n) : P, { close: closed, stroke: o.stroke, lw: o.lw || 1.5, glow: o.glow, alpha: o.alpha }));
      if (!closed) { R.line(A[0], B[0], { stroke: o.stroke, lw: o.lw || 1.5 }); R.line(A[n], B[n], { stroke: o.stroke, lw: o.lw || 1.5 }); }
    }

    /* ---------- pare-feu ---------- */
    function gate(R, s) {
      const h = 4.0, d = 2.6;
      R.box([GX, 0, 0], [0.55, 2 * h, 2 * d], { shade: true, fa: FA * 0.7, tint: T.accent, stroke: R.A(0.55), lw: 1.3 });
      const sweep = (s.clock * 0.6) % 1; // reflet qui glisse sur la membrane
      R.line([GX, -h + sweep * 2 * h, -d], [GX, -h + sweep * 2 * h, d], { stroke: R.A(0.35), lw: 1.5, glow: 10 });
      R.text([GX + 0.3, h - 0.15, 1.8], "pare-feu", { px: 17, weight: 800, ls: 1, align: "left", font: "display", color: R.Wh(0.97) });
      DR.surfaces.forEach((sf, i) => {
        const y = PORTY[i], on = sf.id === "MCP" && (s.ph === "send" || s.ph === "mcp");
        R.box([GX, y, 1.5], [0.85, 0.42, 0.9], { shade: true, fa: on ? 0.5 : 0.08, tint: on ? T.accent : ink, stroke: on ? R.A(1) : R.Wh(0.45) });
        R.text([GX + 0.35, y, 1.8], sf.id, { px: 11.5, font: "mono", weight: 700, align: "left", color: on ? R.A(1) : R.Wh(0.6) });
      });
    }

    /* ---------- volumes (coffre, destinataire, agent) ---------- */
    function vaultShape(R, c, o, scaleH = 1) {
      const h = VSZ[1] * scaleH;
      if (T.vault === "cyl") R.prism(c, 1.0, h, 40, o);
      else if (T.vault === "hex") R.prism(c, 1.1, h, 6, o, 0.52);
      else R.box(c, [VSZ[0], h, VSZ[2]], Object.assign({ shade: true }, o));
    }
    function bracket(R, c, hw, hh, label) {
      if (!T.reticle) return;
      const pts = [[-1, -1], [1, -1], [1, 1], [-1, 1]].map(([a, b]) => R.p([c[0] + a * hw, c[1] + b * hh, c[2] + 1]));
      if (pts.some((p) => !p.ok)) return;
      R.raw(R.p(c).z - 1, (ctx) => {
        const xs = pts.map((p) => p.x), ys = pts.map((p) => p.y), x0 = Math.min(...xs), x1 = Math.max(...xs), y0 = Math.min(...ys), y1 = Math.max(...ys), k = 14;
        ctx.strokeStyle = R.A(0.9); ctx.lineWidth = 2; ctx.beginPath();
        [[x0, y0, 1, 1], [x1, y0, -1, 1], [x1, y1, -1, -1], [x0, y1, 1, -1]].forEach(([x, y, dx, dy]) => { ctx.moveTo(x + dx * k, y); ctx.lineTo(x, y); ctx.lineTo(x, y + dy * k); });
        ctx.stroke();
        if (label) { ctx.font = `700 10px ${DR.MONO}`; ctx.fillStyle = R.A(0.9); ctx.textAlign = "left"; ctx.fillText(label, x0, y0 - 7); }
      });
    }
    function vault(R, s, twin) {
      const VN = DR.vaultNow(s);
      const c = twin ? VB : VA, a = twin ? tw(s) : 1, sg = twin ? -1 : 1, fr = VN.frozen && !twin;
      if (!twin) { R.shadow(c, 1.7, VSZ[1], 0.65); R.pool([c[0], 0.01, c[2]], 2.6, fr ? ink : T.accent, 0.22); }
      vaultShape(R, c, { fa: fr ? 0.3 : FA * (twin ? 0.8 : 1), tint: twin ? T.accent : ink, stroke: fr ? R.Wh(1) : twin ? R.A(0.65 * a) : R.Wh(0.7), lw: fr ? 2.2 : 1.4, alpha: twin ? Math.max(0.35, a) : 1 });
      // réserve
      let bal = VN.balance;
      if (twin && ((s.ph === "sandbox" && s.sub >= 4) || s.reached("effects", s.lastPass))) {
        const P = s.scView.passes[s.lastPass], out = P && P.effects[0];
        if (out && out.acc === "vault" && out.u === "NEAR") bal += out.d;
      }
      const ratio = VN.balance > 0n ? Number((bal * 1000n) / DR.vault.balance) / 1000 : 0;
      const hh = (VSZ[1] - 0.2) * C(ratio), base = c[1] - sg * (VSZ[1] / 2 - 0.1), lc = [c[0], base + (sg * hh) / 2, c[2]];
      const lo = { fa: twin ? 0.22 * a : 0.14, tint: twin ? T.accent : ink, stroke: twin ? R.A(0.35 * a) : R.Wh(0.25) };
      if (T.vault === "cyl") R.prism(lc, 0.82, Math.max(0.02, hh), 40, lo);
      else if (T.vault === "hex") R.prism(lc, 0.9, Math.max(0.02, hh), 6, lo, 0.52);
      else R.box(lc, [VSZ[0] - 0.3, Math.max(0.02, hh), VSZ[2] - 0.3], Object.assign({ shade: true }, lo));
      if (twin) {
        R.text([c[0], c[1] - VSZ[1] / 2 - 0.5, 1.0], "jumeau", { px: 15, weight: 800, font: "display", color: R.A(0.95 * a) });
        if (s.ph === "sandbox" && s.sub >= 1) {
          const p = [c[0] + 1.25, c[1] - 0.2, 1.0], f = s.sub === 1 ? C(s.subP * 3) : C(1 - (s.sub - 1 + s.subP) * 0.7);
          R.halo(p, 0.5, T.accent, 0.6 * f);
          R.text([p[0] + 0.35, p[1], p[2]], "clé jetable", { px: 13, font: "mono", weight: 700, align: "left", color: R.A(f) });
        }
        return;
      }
      // fenêtre 24 h et journal (pur visuel)
      const used = VN.dailyCap > 0n ? Math.round(Number((VN.spent * 24n) / VN.dailyCap)) : 0;
      for (let i = 0; i < 24; i++) { const a0 = (i / 24) * Math.PI * 2 + s.clock * 0.05, a1 = a0 + ((Math.PI * 2) / 24) * 0.68, on = i < used; thick(R, [c[0], c[1] + 0.3, c[2]], 1.55, "y", a0, a1, 0.24, { seg: 60, fa: on ? 0.65 : 0.1, tint: on ? T.accent : ink, stroke: on ? R.A(1) : R.Wh(0.3), lw: 1.1, glow: on ? 8 : 0 }); }
      for (let i = 0; i < 100; i++) { const an = (i / 100) * Math.PI * 2; R.dot([c[0] + Math.cos(an) * 1.9, 0.02, c[2] + Math.sin(an) * 1.9], 0.035, { fill: i < VN.journal ? R.A(1) : R.Wh(0.15) }); }
      // libellés : peu, gros
      const lx = c[0] - 1.8;
      R.text([lx, 2.3, 1], "coffre", { px: 16, weight: 800, font: "display", align: "right", color: R.Wh(0.95) });
      R.text([lx, 1.85, 1], DR.fmt(VN.balance, 0) + " NEAR", { px: 24, weight: 800, font: "mono", align: "right", color: fr ? R.Wh(1) : R.A(1), glow: 10 });
      R.text([lx, 1.42, 1], "2 / tx · 10 / 24 h", { px: 12.5, font: "mono", align: "right", color: R.Wh(0.6) });
      const gelOn = !!s.scView.freeze;
      [["execute", 1.2, false], ["freeze", 0.88, gelOn]].forEach(([k, y, on]) => {
        const p = [c[0] - 0.6, y, c[2] + 1.0];
        R.dot(p, 0.08, { fill: on ? R.Wh(1) : R.A(0.95), glow: on ? 16 : 6 });
        R.text([p[0] + 0.2, y, p[2]], k, { px: 11.5, font: "mono", align: "left", color: on ? R.Wh(1) : R.Wh(0.75) });
      });
      R.text([c[0] - 0.68, 0.56, c[2] + 1.0], "0 FullAccess", { px: 11, font: "mono", align: "left", color: R.Wh(0.45) });
      if (fr) R.text([c[0], c[1] + 0.35, c[2] + 1.1], "FROZEN", { px: 28, weight: 900, ls: 5, font: "display", color: R.Wh(1), glow: 22, glowColor: R.Wh(0.7) });
      bracket(R, c, 1.1, 1.3, "VAULT");
      agent(R, s, c);
    }
    function agent(R, s, c) {
      const ag = [c[0], 3.7, 0], dim = s.scView.bypass ? 0.3 : 1, pulse = s.ph === "arrive" && !s.scView.bypass ? 0.5 + 0.5 * Math.sin(s.clock * 6) : 0;
      const sp = s.clock * 0.8, style = T.agent || "octa";
      R.halo(ag, 1.1, T.accent, (0.25 + pulse * 0.3) * dim);
      if (style === "octa") {
        const r = 0.42, rot = (v) => [v[0] * Math.cos(sp) - v[2] * Math.sin(sp), v[1], v[0] * Math.sin(sp) + v[2] * Math.cos(sp)];
        const P6 = [[0, r * 1.45, 0], [0, -r * 1.45, 0], [r, 0, 0], [-r, 0, 0], [0, 0, r], [0, 0, -r]].map((v) => DR.v.add(ag, rot(v)));
        [[0, 2, 4], [0, 4, 3], [0, 3, 5], [0, 5, 2], [1, 2, 4], [1, 4, 3], [1, 3, 5], [1, 5, 2]].forEach((f) => R.poly(f.map((i) => P6[i]), { fill: R.A((0.14 + pulse * 0.25) * dim), stroke: R.A(0.95 * dim), lw: 1.1 }));
      } else if (style === "orb") {
        R.dot(ag, 0.38, { fill: R.A(0.18 * dim), stroke: R.A(0.95 * dim), lw: 1.4 });
        R.circle(ag, 0.62, "y", { stroke: R.A(0.7 * dim), lw: 1.2 }, 48);
        R.dot([ag[0] + Math.cos(sp * 2) * 0.62, ag[1], ag[2] + Math.sin(sp * 2) * 0.62], 0.07, { fill: R.Wh(dim) });
      } else if (style === "ring") {
        const p1 = R.ring(ag, 0.5, "y", 48).map((v) => [v[0], ag[1] + (v[2] - ag[2]) * Math.sin(sp), ag[2] + (v[2] - ag[2]) * Math.cos(sp)]);
        R.path(p1, { stroke: R.A(0.95 * dim), lw: 1.6 });
        R.circle(ag, 0.5, "z", { stroke: R.A(0.6 * dim), lw: 1.2 }, 48);
        R.dot(ag, 0.12, { fill: R.A(dim), glow: 14 });
      } else {
        const r = 0.33, rot = (v) => [v[0] * Math.cos(sp) - v[2] * Math.sin(sp), v[1], v[0] * Math.sin(sp) + v[2] * Math.cos(sp)];
        const V = [[-r, -r, -r], [r, -r, -r], [r, r, -r], [-r, r, -r], [-r, -r, r], [r, -r, r], [r, r, r], [-r, r, r]].map((v) => DR.v.add(ag, rot(v)));
        [[0, 1], [1, 2], [2, 3], [3, 0], [4, 5], [5, 6], [6, 7], [7, 4], [0, 4], [1, 5], [2, 6], [3, 7]].forEach(([i, j]) => R.line(V[i], V[j], { stroke: R.A(0.95 * dim), lw: 1.4 }));
        if (!T.wire) [[0, 1, 2, 3], [4, 5, 6, 7], [0, 1, 5, 4], [3, 2, 6, 7], [0, 3, 7, 4], [1, 2, 6, 5]].forEach((f, fi) => R.poly(f.map((i) => V[i]), { fill: R.A((0.1 + 0.08 * (fi % 3)) * dim) }));
      }
      R.text([ag[0], ag[1] + 0.95, 0], s.scView.bypass ? "agent · clé volée" : "agent IA", { px: 16, weight: 800, font: "display", color: R.Wh(0.25 + 0.72 * dim) });
      R.line([ag[0], ag[1] - 0.7, 0], [c[0], c[1] + VSZ[1] / 2, 0], { stroke: R.A(0.3 * dim), dash: [3, 5] });
    }
    function recipient(R, s, twin) {
      const S = s.scView;
      if (!S.to) return;
      const c = twin ? RB : RA, a = (twin ? tw(s) : 1) * appear(s), sg = twin ? -1 : 1;
      const o = { fa: FA * 0.9, tint: twin ? T.accent : ink, stroke: twin ? R.A(0.7 * a) : R.Wh(0.65 * a), lw: 1.3, alpha: Math.max(0.3, a) };
      const got = !twin && s.reached("proof");
      if (!twin) { R.shadow(c, 1.5, 2.3, 0.55); R.pool([c[0], 0.01, c[2]], 2.3, got ? T.accent : ink, got ? 0.3 : 0.08); }
      if (S.to.kind === "dex") R.prism(c, 0.95, 2.3, 6, o, 0.5 + s.clock * 0.15);
      else if (S.to.kind === "attacker") {
        const ap = [c[0], c[1] + sg * 1.3, c[2]], b = c[1] - sg * 1.15, sp = s.clock * 0.3;
        const B = [0, 1, 2, 3].map((i) => { const an = sp + (i / 4) * Math.PI * 2; return [c[0] + Math.cos(an) * 0.95, b, c[2] + Math.sin(an) * 0.95]; });
        for (let i = 0; i < 4; i++) R.poly([ap, B[i], B[(i + 1) % 4]], { fill: R.Wh(0.05 * a), stroke: o.stroke, lw: 1.3 });
      } else {
        R.box(c, [1.9, 2.3, 1.9], Object.assign({ shade: true }, o));
        R.box(c, [0.8, 1.0, 0.8], { fill: "rgba(0,0,0,0.75)", stroke: o.stroke });
      }
      if (!twin) {
        R.text([c[0], 3.4, 1.0], S.to.short, { px: 19, weight: 800, font: "display", color: R.Wh(0.97 * a) });
        R.text([c[0], 2.98, 1.0], S.to.type, { px: 12.5, font: "mono", color: R.Wh(0.55 * a) });
        if (got && S.passes.length) {
          const P = S.passes[S.passes.length - 1], g = P.effects.find((e) => e.acc !== "vault");
          if (g) R.text([c[0], c[1], 1.3], "+" + DR.fmt(g.d, 2) + " " + g.u, { px: 22, font: "mono", weight: 800, color: R.A(C(s.since("proof") / 0.4)), glow: 16 });
        }
        bracket(R, c, 1.05, 1.25, S.to.type.toUpperCase());
      } else if (s.reached("sandbox") && (s.ph !== "sandbox" || s.sub >= 3)) {
        [0, 1].forEach((k) => { const ok = !(S.id === "drainer" && k === 1); R.text([c[0], c[1] + 0.45 - k * 0.4, 1.05], ok ? "reçu ✓" : "0 TEST", { px: 13, font: "mono", weight: 700, color: ok ? R.A(0.95 * a) : R.Wh(0.85 * a) }); });
      }
    }

    /* ---------- chaîne ---------- */
    function rail(R, s) {
      const n = 16, run = s.ph === "chain" ? E(C((s.p - 0.12) / 0.88)) : s.reached("chain") ? 1 : 0;
      R.box([(RAIL[0] + RAIL[1]) / 2, RY - 0.32, -0.3], [RAIL[1] - RAIL[0] + 0.7, 0.1, 0.6], { shade: true, fa: FA * 0.6, tint: ink, stroke: R.Wh(0.2) }); // le rail qui porte les blocs
      for (let i = 0; i < n; i++) {
        const x = L(RAIL[0], RAIL[1], i / (n - 1)), u = i / (n - 1), on = run > u;
        const pop = on && s.ph === "chain" ? 1 + 0.35 * Math.max(0, 1 - (run - u) * 8) : 1;
        R.box([x, RY, -0.3], [0.38 * pop, 0.38 * pop, 0.38 * pop], { shade: true, fa: on ? 0.55 : FA * 0.6, tint: on ? T.accent : ink, stroke: on ? R.A(1) : R.Wh(0.25), glow: on ? 8 : 0 });
      }
      R.text([L(RAIL[0], RAIL[1], 0.62), RY + 0.5, -0.3], "chaîne NEAR", { px: 12, font: "mono", color: R.Wh(0.45) });
      const blk = s.since("blocked") >= 0 ? s.since("blocked") : s.since("onchain");
      if (blk >= 0) R.text([L(RAIL[0], RAIL[1], 0.62), RY + 0.95, -0.3], "intacte", { px: 18, weight: 900, ls: 3, font: "display", color: R.Wh(C(blk / 0.6)) });
    }

    /* ---------- bac à sable : 5 anneaux ---------- */
    function stations(R, s) {
      const inSand = s.ph === "sandbox";
      DR.sandboxSteps.forEach((st, i) => {
        const done = s.reached("sandbox") && (!inSand || s.sub > i || (s.sub === i && s.subP > 0.35)), cur = inSand && s.sub === i;
        R.circle(ST[i], 0.58, "x", { fill: T.wire ? null : done ? R.A(0.1) : R.Wh(0.012) }, 44);
        thick(R, ST[i], 0.58, "x", 0, Math.PI * 2, 0.4, { fa: done ? 0.42 : 0.12, stroke: done ? R.A(0.95) : R.A(0.35), lw: cur ? 2.6 : 1.3, glow: cur ? 16 : 0 });
        if (cur) { const a0 = s.clock * 4; R.circle(ST[i], 0.78, "x", { stroke: R.A(0.9), lw: 2.5, glow: 12 }, 16, a0, a0 + 1.4); }
        const lbl = ["clone", "clé", "joué", "reçus", "diff"][i];
        R.text([ST[i][0], -2.5, 0.6], lbl, { px: cur ? (T.upper ? 14 : 17) : T.upper ? 10.5 : 12.5, ls: T.upper ? -0.6 : 0, weight: cur ? 800 : 600, font: cur ? "display" : "body", color: cur ? R.A(1) : R.Wh(done ? 0.85 : 0.4), glow: cur ? 10 : 0 });
      });
      [IN, ...ST].forEach((p, i, a) => i && R.line(a[i - 1], p, { stroke: R.A(0.28), dash: [2, 4] }));
    }

    /* ---------- le motif ---------- */
    function ticket(R, s) {
      const S = s.scView;
      if (!S.motif) return;
      const a = appear(s) * (s.ph === "mcp" ? 0.35 : 1), dim = s.reached("send") || S.bypass ? 0.6 : 1, c = TK, w = TW, h = TH;
      const sl = O(C(Math.max(0, s.since("arrive")) / 0.7) + (s.scViewI === 0 ? 1 : 0)), cc = [c[0], c[1] + (1 - sl) * 0.6, c[2]];
      R.box(cc, [w, h, 0.5], { shade: true, fa: T.wire ? 0 : 0.09, tint: ink, stroke: S.bypass ? R.Wh(0.75 * a) : R.A(0.8 * a * dim), lw: 1.3, alpha: a });
      const m = S.motif, it = S.intent || { beneficiaries: "—", min_in: "—" }, x0 = cc[0] - w / 2 + 0.2;
      const o = { size: 0.175, min: 11, max: 15, font: "mono", align: "left", color: R.Wh(0.92 * a * dim), zb: -0.2 };
      R.text([x0, cc[1] + 0.62, cc[2] + 0.28], "→ " + m.receiver, { ...o, weight: 700, color: S.bypass ? R.Wh(a) : R.A(a * dim) });
      R.text([x0, cc[1] + 0.24, cc[2] + 0.28], m.method + "  " + m.args, o);
      R.text([x0, cc[1] - 0.14, cc[2] + 0.28], m.deposit + " · " + m.gas, o);
      R.text([x0, cc[1] - 0.58, cc[2] + 0.28], "min " + it.min_in + " · " + it.beneficiaries, { ...o, color: R.Wh(0.6 * a * dim) });
      R.text([cc[0], cc[1] + h / 2 + 0.42, cc[2]], S.say, { size: 0.21, min: 13, max: 19, weight: 600, font: "display", color: R.Wh(0.92 * a) });
    }

    /* ---------- la demande en vol ---------- */
    const rs = [VA[0] + 1.25, RY, -0.3], re = [RA[0] - 1.2, RY, -0.3];
    function orbPos(s) {
      const S = s.scView;
      if (!S.passes.length) return null;
      const ph = s.ph;
      if (ph === "send") { const k = E(s.p); return k < 0.45 ? M(TK, PORT, k / 0.45) : M(PORT, IN, (k - 0.45) / 0.55); }
      if (ph === "sandbox") { const pts = [IN, ...ST]; return M(pts[s.sub], pts[s.sub + 1], E(C(s.subP * 1.8))); }
      if (["effects", "rules", "verdict", "approve"].includes(ph)) return ST[4];
      if (ph === "cert") return M(ST[4], SEAL, E(C(s.p * 3)));
      if (ph === "chain") return s.p < 0.12 ? M(SEAL, rs, s.p / 0.12) : M(rs, re, E((s.p - 0.12) / 0.88));
      if (ph === "proof") return re;
      return null;
    }
    function orb(R, s) {
      const S = s.scView;
      if (!S.passes.length) return;
      if (s.ph === "blocked") { const k = O(C(s.p * 1.4)); SHARDS.forEach((d) => R.dot(DR.v.add(ST[4], DR.v.mul(d, k * 3)), 0.055, { fill: R.Wh(1 - k) })); return; }
      const p = orbPos(s);
      if (!p) return;
      const al = s.ph === "approve" ? 1 - s.p : s.ph === "proof" ? 1 - s.p : 1;
      const style = T.orb || "comet";
      if (style === "comet") {
        const tr = [];
        for (let i = 12; i >= 1; i--) { const q = orbPos(Object.assign(DR.at(Math.max(0, s.t - i * 0.03)), {})); if (q) tr.push(q); }
        tr.push(p); R.trail(tr, T.accent, 7, 0.7 * al);
      }
      if (style === "pulse") for (let i = 0; i < 3; i++) { const r = ((s.clock * 0.9 + i / 3) % 1) * 0.9; R.circle(p, 0.2 + r, "z", { stroke: R.A(al * (1 - r / 0.9)), lw: 1.5 }, 32); }
      if (style === "square") { const sp = s.clock * 2, r = 0.22; const V = [[-r, -r], [r, -r], [r, r], [-r, r]].map(([x, y]) => [p[0] + x * Math.cos(sp) - y * Math.sin(sp), p[1] + x * Math.sin(sp) + y * Math.cos(sp), p[2]]); R.poly(V, { stroke: R.A(al), lw: 2, fill: R.A(0.2 * al) }); }
      R.halo(p, 0.9, T.accent, 0.55 * al, { zb: -1 });
      R.dot(p, 0.13, { fill: R.Wh(al), zb: -1.1, glow: 20 });
    }

    /* ---------- effets ---------- */
    function arcs(R, s) {
      const S = s.scView, P = S.passes.length ? S.passes[s.lastPass] : null;
      if (!P) return;
      const out = P.effects.find((e) => e.acc === "vault" && e.d < 0n), inn = P.effects.find((e) => e.acc === "vault" && e.u !== "NEAR");
      const fl = [];
      if (out) fl.push({ a0: VB[0] + 1.05, a1: RB[0] - 1.05, depth: 3.05, z: 0.4, label: "− " + DR.fmt(-out.d, 2) + " NEAR", lx: -5.6 });
      if (inn) fl.push({ a0: RB[0] - 1.05, a1: VB[0] + 1.05, depth: 3.65, z: 0.9, label: (inn.d === 0n ? "0 " : "+ " + DR.fmt(inn.d, 2) + " ") + inn.u, lx: -4.3, zero: inn.d === 0n });
      const draw = (sg, k, al, real) => fl.forEach((f) => {
        const n = 56, pts = [];
        for (let i = 0; i <= Math.round(n * k); i++) { const u = i / n; pts.push([L(f.a0, f.a1, u), sg * (0.5 + Math.sin(u * Math.PI) * (f.depth - 0.5)), f.z]); }
        if (pts.length < 2) return;
        // le flux est un RUBAN (largeur en profondeur), plus une simple ligne
        const wz = 0.22, e1 = pts.map((p) => [p[0], p[1], p[2] - wz]), e2 = pts.map((p) => [p[0], p[1], p[2] + wz]);
        if (!f.zero && !T.wire) for (let i = 0; i + 1 < pts.length; i += 4) { const j = Math.min(pts.length - 1, i + 4); R.poly([...e1.slice(i, j + 1), ...e2.slice(i, j + 1).reverse()], { fill: R.A(0.3 * al) }); }
        [e1, e2].forEach((e) => R.path(e, { stroke: f.zero ? R.Wh(0.65 * al) : R.A(al), lw: 2, dash: f.zero ? [5, 7] : null, glow: real ? 14 : 5 }));
        if (k >= 1 && !f.zero) for (let j = 0; j < 3; j++) { const u = (s.clock * 0.35 + j / 3) % 1; R.dot([L(f.a0, f.a1, u), sg * (0.5 + Math.sin(u * Math.PI) * (f.depth - 0.5)), f.z], 0.1, { fill: R.Wh(al), glow: 12 }); }
        if (k > 0.6 && !real) { const u = (f.lx - f.a0) / (f.a1 - f.a0); R.text([f.lx, sg * (0.5 + Math.sin(u * Math.PI) * (f.depth - 0.5)) - sg * 0.38, f.z], f.label, { px: 19, font: "mono", weight: 800, color: f.zero ? R.Wh(0.95 * al) : R.A(al), glow: f.zero ? 0 : 8 }); }
      });
      if (s.reached("effects", s.lastPass)) draw(-1, O(C(s.since("effects", s.lastPass) / 1.1)), tw(s), false);
      if (s.reached("chain")) draw(1, O(C(s.since("chain") / 1.5)), 1, true);
      if (s.reached("proof")) R.text([0.6, 0.9, 1.2], "simulé = réel", { px: 40, weight: 900, ls: 2, font: "display", color: R.A(O(C(s.since("proof") / 0.6))), glow: 30 });
    }

    /* ---------- règles ---------- */
    function rules(R, s) {
      const x0 = -2.4, x1 = 4.4, z = 3.2, S = s.scView, scanning = s.ph === "rules" ? s.p : -1;
      DR.rules.forEach((r, i) => {
        const x = L(x0, x1, i / 11), bit = s.rules.includes(r.id), ok5 = r.id === "R5" && S.passes.length > 1 && s.reached("approve");
        const near = scanning >= 0 && Math.abs(scanning * 1.08 - (i + 0.5) / 12) < 0.05;
        // chaque règle est un plot posé sur la dalle ; celui qui mord se dresse
        const hot = bit && !ok5, hgt = hot ? 0.3 + 0.9 * O(C(s.since("rules", s.lastPass) / 0.9 + 0.15)) : near ? 0.6 : 0.2;
        R.box([x, hgt / 2, z - 0.15], [0.32, hgt, 0.32], { shade: true, fa: hot ? 0.65 : near ? 0.5 : 0.16, tint: hot ? ink : T.accent, stroke: hot ? R.Wh(1) : near ? R.A(1) : R.A(0.5), lw: 1.2 });
        R.text([x, -0.32, z], r.id, { px: bit ? 15 : 11.5, font: "mono", weight: bit ? 800 : 500, color: bit && !ok5 ? R.Wh(1) : near || ok5 ? R.A(1) : R.Wh(0.45), glow: bit && !ok5 ? 14 : 0, glowColor: R.Wh(0.7) });
        if (bit && !ok5) R.text([x, -0.72, z], r.k, { px: 16, weight: 800, font: "display", color: R.Wh(1) });
        if (ok5 && s.lastPass === 0) R.text([x, -0.72, z], "approuvé", { px: 14, weight: 700, color: R.A(1) });
      });
      if (scanning >= 0) { const x = L(x0, x1, C(scanning * 1.08)); R.line([x, 0.02, z - 0.7], [x, 0.02, z + 0.3], { stroke: R.A(1), lw: 2.5, glow: 18 }); }
      if (S.diff && s.reached("rules", S.passes.length - 1)) {
        const k = O(C(s.since("rules", S.passes.length - 1) / 0.8)), bx = -7.0, by = -3.0, bz = 1.4;
        R.text([bx + 0.95, by + 0.45, bz], "R12 · ×1 / ×1/100", { px: 12, font: "mono", color: R.Wh(0.65 * k) });
        [[S.diff.full], [S.diff.small]].forEach(([v], j) => {
          const y = by - j * 0.4;
          const ln = Math.max(0.03, 1.9 * (v / 1000) * k);
          R.box([bx + 0.95, y, bz], [1.9, 0.22, 0.22], { shade: true, fa: 0.05, tint: ink, stroke: R.Wh(0.2) });
          R.box([bx + ln / 2, y, bz], [ln, 0.22, 0.22], { shade: true, fa: 0.7, tint: T.accent, stroke: R.A(1) });
          R.text([bx + 2.1, y, bz], v + " ‰", { px: 13, font: "mono", weight: 700, align: "left", color: R.A(0.95) });
        });
      }
    }

    function verdict(R, s) {
      if (!s.verdict || T.verdictHud) return;
      const k = O(C(s.since("verdict", s.lastPass) / 0.5)), v = s.verdict;
      const fade = s.reached("chain") ? 1 - C(s.since("chain") / 0.5) : s.ph === "approve" ? 1 - s.p * 0.7 : 1;
      const c = [0.6, 1.05 + (1 - k) * 0.4, 0.6], o = { px: L(40, T.verdictPx || 88, k), weight: T.verdictWeight || 900, ls: T.verdictLs == null ? 12 : T.verdictLs, font: "display", zb: -20, alpha: fade };
      if (v === "ALLOW") R.text(c, "ALLOW", { ...o, color: R.A(1), glow: 36 });
      if (v === "WARN") R.text(c, "WARN", { ...o, color: R.Wh(0.95), outline: 2, alpha: fade * (0.75 + 0.25 * Math.sin(s.clock * 6)) });
      if (v === "BLOCK") R.text(c, "BLOCK", { ...o, color: R.Wh(1), glow: 20, glowColor: R.Wh(0.5) });
    }

    function actor(R, label, sub, shape) {
      const p = ACT;
      R.halo(p, 1.0, ink, 0.18);
      if (shape === "owner") { R.dot(p, 0.22, { fill: R.Wh(0.95), glow: 14, glowColor: R.Wh(0.6) }); R.circle([p[0], p[1] - 0.55, p[2]], 0.34, "z", { stroke: R.Wh(0.85), lw: 1.8 }, 20, Math.PI, Math.PI * 2); }
      if (shape === "thief") { const sp = [[p[0], p[1] + 0.6, p[2]], [p[0] - 0.4, p[1] - 0.4, p[2] - 0.3], [p[0] + 0.4, p[1] - 0.4, p[2] - 0.3], [p[0], p[1] - 0.4, p[2] + 0.4]]; [[0, 1, 2], [0, 2, 3], [0, 3, 1]].forEach((f) => R.poly(f.map((i) => sp[i]), { fill: R.Wh(0.12), stroke: R.Wh(0.95), lw: 1.4 })); }
      if (shape === "phone") R.box(p, [0.6, 1.05, 0.26], { shade: true, fa: 0.12, tint: ink, stroke: R.Wh(0.95), lw: 1.5 });
      R.text([p[0], p[1] + 0.95, p[2]], label, { px: 17, weight: 800, font: "display", color: R.Wh(0.97) });
      if (sub) R.text([p[0], p[1] - 1.1, p[2]], sub, { px: 12, font: "mono", color: R.Wh(0.65) });
    }
    function approve(R, s) {
      const S = s.scView;
      if (S.passes.length < 2 || !s.reached("approve") || (s.reached("sandbox", 1) && s.since("sandbox", 1) > 1.5)) return;
      const k = s.ph === "approve" ? s.p : 1;
      actor(R, "propriétaire", "approve_contract · 1 yocto", "owner");
      R.line(ACT, M(ACT, HIT, E(k)), { stroke: R.Wh(0.85), lw: 2, dash: [6, 5] });
      R.halo(M(ACT, HIT, E(k)), 0.4, ink, 0.8);
    }
    function cert(R, s) {
      if (!s.reached("cert") || (s.reached("chain") && s.since("chain") > 0.4)) return;
      const inC = s.ph === "cert", n = inC ? s.sub + (s.subP > 0.5 ? 1 : 0) : 7, rot = s.clock * 0.6;
      for (let i = 0; i < 7; i++) { const a0 = rot - Math.PI / 2 + (i / 7) * Math.PI * 2 + 0.06, a1 = a0 + (Math.PI * 2) / 7 - 0.12, on = i < n; thick(R, SEAL, 0.8, "z", a0, a1, 0.4, { seg: 42, fa: on ? 0.6 : 0.08, tint: on ? T.accent : ink, stroke: on ? R.A(1) : R.Wh(0.28), lw: 1.8, glow: on ? 12 : 0 }); }
      R.halo(SEAL, 1.2, T.accent, 0.25);
      R.text([SEAL[0] + 1.05, SEAL[1] + 0.18, SEAL[2]], s.certLabel || "certificat", { px: 16, weight: 800, font: "display", align: "left", color: R.Wh(0.95) });
      R.text([SEAL[0] + 1.05, SEAL[1] - 0.22, SEAL[2]], inC ? (s.sub + 1) + "/7 · " + DR.certChecks[s.sub].k : "7/7 · Ed25519", { px: 13, font: "mono", align: "left", color: R.A(1) });
    }
    function mcp(R, s) {
      const S = s.scView;
      if (!S.mcp || !s.reached("mcp")) return;
      const k = s.ph === "mcp" ? s.p : 1, a = [GX - 0.1, PORTY[2], 1.5], b = [VA[0] + 0.4, 3.6, 0.3], pts = [];
      for (let i = 0; i <= 48; i++) { const u = i / 48, p = M(a, b, u); p[1] += Math.sin(u * Math.PI) * 2.3; p[2] += Math.sin(u * Math.PI) * 1.2; pts.push(p); }
      R.path(pts, { stroke: R.Wh(0.85), lw: 2, dash: [7, 6] });
      R.halo(pts[Math.min(48, Math.floor(E(k) * 48))], 0.45, ink, 0.9);
      R.text([-11.6, 5.75, 1.0], "boucle de l’agent · MCP", { align: "left", px: 17, weight: 800, font: "display", color: R.Wh(0.97) });
      R.text([-11.6, 5.35, 1.0], "wallet_explain · le pare-feu ne reformule jamais", { align: "left", px: 12, font: "mono", color: R.Wh(0.6) });
    }
    function theft(R, s) {
      const S = s.scView;
      if (!S.bypass) return;
      actor(R, "voleur", "clé d’agent volée", "thief");
      const kb = s.ph === "bypass" ? E(s.p) : s.reached("onchain") ? 1 : 0;
      if (kb > 0) R.line(ACT, M(ACT, HIT, kb), { stroke: R.Wh(0.95), lw: 2.4, dash: [7, 5] });
      if (s.reached("onchain")) {
        const k = C(s.since("onchain") / 0.4);
        R.text([VA[0], VA[1] + 0.3, 1.3], S.onchainErr, { px: 19, font: "mono", weight: 800, color: R.Wh(k), glow: 20, glowColor: R.Wh(0.6) });
        R.text([VA[0], VA[1] - 0.15, 1.3], "refus on-chain", { px: 13, weight: 700, color: R.Wh(0.8 * k) });
      }
    }
    function freeze(R, s) {
      if (!s.scView.freeze) return;
      const k = s.k("freeze");
      actor(R, "téléphone", "clé de gel", "phone");
      R.line(ACT, M(ACT, HIT, E(C(k * 2.2))), { stroke: R.Wh(1), lw: 2.4, glow: 14, glowColor: R.Wh(0.6) });
      if (k > 0.5) R.text([VA[0], VA[1] - 0.95, 1.3], "ERR_FROZEN", { px: 14, font: "mono", weight: 700, color: R.Wh(0.85) });
    }

    /* ---------- banc ---------- */
    function bench(R, s) {
      if (s.ph !== "bench" && s.ph !== "end") return;
      const b = s.bench, cx = BX, sp = 0.3;
      const slot = (n, up) => [cx - 1.35 + (n % 10) * sp, up ? 0.9 + Math.floor(n / 10) * sp : -0.9 - Math.floor(n / 10) * sp, 0];
      R.box([cx, 2.25, 0], [3.3, 3.2, 0.45], { shade: true, fa: FA * 0.4, tint: ink, stroke: R.Wh(0.18) });
      R.box([cx, -2.25, 0], [3.3, 3.2, 0.45], { shade: true, fa: FA * 0.5, tint: T.accent, stroke: R.A(0.3) });
      let up = 0, dn = 0;
      DR.benchRobots.forEach((rb, j) => {
        const idx = rb.atk ? dn++ : up++, k = C((b - (j / 200) * 0.78) / 0.18);
        if (k <= 0) return;
        const tg = slot(idx, !rb.atk), st = [cx + (((j * 37) % 9) - 4) * 0.35, 5.6, (((j * 13) % 5) - 2) * 0.3], to = [tg[0], 0, 0];
        R.dot(k < 0.5 ? M(st, to, k * 2) : M(to, tg, (k - 0.5) * 2), 0.085, { fill: rb.atk ? R.Wh(0.92) : R.A(1), glow: k < 1 ? 8 : 0 });
      });
      const a = Math.max(0.35, C((b - 0.75) / 0.25)), mx = cx + 1.95;
      R.text([cx, 4.45, 0], "200 robots", { px: 22, weight: 900, font: "display", color: R.Wh(0.97) });
      R.text([mx, 2.5, 0], "100 honnêtes", { px: 17, weight: 800, font: "display", align: "left", color: R.A(a) });
      R.text([mx, 2.05, 0], "TN 100 · FP 0", { px: 13, font: "mono", align: "left", color: R.A(a) });
      R.text([mx, -2.0, 0], "100 attaques", { px: 17, weight: 800, font: "display", align: "left", color: R.Wh(a) });
      R.text([mx, -2.45, 0], "TP 100 · FN 0", { px: 13, font: "mono", align: "left", color: R.Wh(a) });
      R.text([mx, 0.3, 0], "F1 1,00", { px: 30, font: "mono", weight: 900, align: "left", color: R.A(a), glow: 16 });
      R.text([mx, -0.25, 0], "précision 1,00 · rappel 1,00 · 2 µs p95", { px: 12, font: "mono", align: "left", color: R.Wh(0.6 * a) });
      R.text([mx, -0.6, 0], DR.traps.map((t) => t.id).join(" · "), { px: 11, font: "mono", align: "left", color: R.Wh(0.4 * a) });
    }

    const cam = Object.assign({ yaw: -0.32, pitch: 0.2, dist: 22.5, fov: 0.98, target: [-1.2, -0.35, 0] }, T.camera || {});
    DR.mount({
      id: T.id, name: T.name, accent: T.accent, bg: T.bg, theme: T, nav: T.nav,
      idea: T.idea, hello: T.hello, legend: T.legend || [
        ["exécuté", `background:${T.accent}`],
        ["WARN", `border:1.5px solid ${ink};box-sizing:border-box`],
        ["BLOCK", `background:linear-gradient(135deg,transparent 42%,${ink} 42% 58%,transparent 58%)`],
      ],
      camera: cam,
      cameraAt(s) {
        const ph = s.ph, P = T.camK || 1;
        const Y = cam.yaw;
        if (ph === "bench" || ph === "end") return { target: [11.8, 0.4, 0], dist: 19, yaw: Y + 0.05 * P, pitch: cam.pitch * 0.6 };
        if (ph === "sandbox" || ph === "effects") return { target: [-0.8, -0.55, 0], dist: cam.dist - 0.5, pitch: cam.pitch * 0.75, yaw: Y + 0.1 * P };
        if (ph === "rules" || ph === "verdict") return { target: [-0.6, -0.5, 0.5], dist: cam.dist - 1, pitch: cam.pitch * 1.1, yaw: Y + 0.08 * P };
        if (ph === "cert" || ph === "chain" || ph === "proof") return { target: [-0.8, 0.3, 0], dist: cam.dist + 0.5, pitch: cam.pitch * 1.4, yaw: Y - 0.06 * P };
        if (["approve", "mcp", "bypass", "onchain", "freeze"].includes(ph)) return { target: [-3.8, 0.2, 0], dist: cam.dist - 0.5, yaw: Y + 0.05 * P, pitch: cam.pitch * 1.05 };
        return {};
      },
      background,
      draw(R, s) {
        particles(R, s); plane(R, s); gate(R, s);
        vault(R, s, false); vault(R, s, true);
        recipient(R, s, false); recipient(R, s, true);
        rail(R, s); stations(R, s); ticket(R, s); orb(R, s); arcs(R, s);
        rules(R, s); verdict(R, s); approve(R, s); cert(R, s); mcp(R, s); theft(R, s); freeze(R, s); bench(R, s);
      },
    });
  };
})();
