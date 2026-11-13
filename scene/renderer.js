/* dry-run · renderer V7 — sur-ensemble compatible de web/scene/renderer.js.
   Ajouts : perspective atmosphérique (brume en profondeur), volumes de verre ombrés
   (faces éclairées selon une lumière), cylindres, prismes, flaques de lumière au sol,
   traînées, texte à graisse/police de thème. Même API : R.p, R.poly, R.path, R.line,
   R.circle, R.box, R.dot, R.text, R.raw, R.flush. Zéro dépendance. */
(function () {
  "use strict";
  const DR = (window.DR = window.DR || {});
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const add = (a, b) => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
  const mul = (a, k) => [a[0] * k, a[1] * k, a[2] * k];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const norm = (a) => { const l = Math.hypot(a[0], a[1], a[2]) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };
  DR.v = { sub, add, mul, dot, cross, norm };
  DR.SANS = '"Inter", "Segoe UI Variable Display", "Segoe UI", system-ui, sans-serif';
  DR.MONO = '"JetBrains Mono", "Cascadia Code", Consolas, ui-monospace, monospace';
  DR.SERIF = '"Iowan Old Style", "Palatino Linotype", Palatino, Georgia, "Times New Roman", serif';
  const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
  DR.rgba = (h, a) => { const c = hex(h); return `rgba(${c[0]},${c[1]},${c[2]},${a})`; };

  DR.Renderer = function (canvas, conf) {
    const ctx = canvas.getContext("2d");
    const T = conf.theme || {};
    const R = { ctx, canvas, W: 0, H: 0, dpr: 1, list: [], T };
    R.A = (a = 1) => DR.rgba(conf.accent, a);
    R.Wh = (a = 1) => (T.ink ? DR.rgba(T.ink, a) : `rgba(255,255,255,${a})`);
    R.BG = conf.bg || "#05070a";
    R.cam = Object.assign({ yaw: 0, pitch: 0.2, dist: 14, fov: 0.85, target: [0, 0, 0] }, conf.camera || {});
    R.user = { yaw: 0, pitch: 0, zoom: 1 };
    R.cy = conf.centerY || 0.44;
    // marges réservées (px) : terminal à droite, mots « réel / sand » à gauche.
    // La scène se cadre et se centre dans l'espace libre entre les deux.
    R.padL = 0; R.padR = 0; R.padT = 0; R.cx0 = 0; R.cy0 = 0;
    R.LIGHT = norm([-0.45, 0.8, 0.55]);
    DR.COND = '"Bahnschrift", "DIN Alternate", "Arial Narrow", "Segoe UI", sans-serif';
    const FONT = { sans: DR.SANS, mono: DR.MONO, serif: DR.SERIF, cond: DR.COND };
    R.fkey = (f) => (f === "body" ? T.font || "sans" : f === "display" ? T.display || T.font || "sans" : f);
    R.font = (f) => FONT[R.fkey(f)] || DR.SANS;
    R.cond = (f) => (R.fkey(f) === "cond" ? "condensed " : "");
    R.lk = T.lineK || 1;
    R.gk = T.glowK == null ? 1 : T.glowK;

    R.resize = () => {
      R.dpr = Math.min(2, window.devicePixelRatio || 1);
      R.W = canvas.clientWidth; R.H = canvas.clientHeight;
      canvas.width = Math.round(R.W * R.dpr); canvas.height = Math.round(R.H * R.dpr);
      R.narrow = R.W < 760;
    };
    R.basis = () => {
      const c = R.cam, yaw = c.yaw + R.user.yaw, pit = DR.clamp(c.pitch + R.user.pitch, -1.3, 1.3);
      const d = c.dist * R.user.zoom * (R.narrow ? 1.45 : 1);
      const eye = [c.target[0] + d * Math.sin(yaw) * Math.cos(pit), c.target[1] + d * Math.sin(pit), c.target[2] + d * Math.cos(yaw) * Math.cos(pit)];
      const f = norm(sub(c.target, eye)), r = norm(cross(f, [0, 1, 0]));
      R.eye = eye; R.fw = f; R.rt = r; R.up = cross(r, f); R.D = d;
      const free = Math.max(120, R.W - R.padL - R.padR); // largeur réellement disponible pour la scène
      R.cx0 = R.padL + free / 2;
      const freeH = Math.max(160, R.H - R.padT); // hauteur sous le HUD du haut
      R.cy0 = R.padT + freeH * R.cy;
      R.F = (Math.min(free, freeH * 1.6) / 2) / Math.tan(c.fov / 2);
    };
    R.p = (x, y, z) => {
      if (Array.isArray(x)) { z = x[2]; y = x[1]; x = x[0]; }
      const v = [x - R.eye[0], y - R.eye[1], z - R.eye[2]];
      const zz = dot(v, R.fw), k = R.F / Math.max(0.05, zz);
      return { x: R.cx0 + dot(v, R.rt) * k, y: R.cy0 - dot(v, R.up) * k, k, z: zz, ok: zz > 0.1 };
    };
    // brume : 1 devant, T.fog derrière (perspective atmosphérique = profondeur lisible)
    const FOG = T.fog == null ? 0.45 : T.fog;
    R.fog = (z) => 1 - (1 - FOG) * DR.clamp((z - R.D * 0.85) / (R.D * 0.9));
    R.add = (z, fn) => R.list.push({ z, fn });
    const st = (o, d) => Object.assign({}, d, o || {});
    const strokeFill = (o) => {
      if (o.glow && R.gk > 0) { ctx.shadowColor = o.glowColor || R.A(0.9); ctx.shadowBlur = o.glow * R.gk; }
      if (o.fill) { ctx.fillStyle = o.fill; ctx.fill(); }
      if (o.stroke) { ctx.strokeStyle = o.stroke; ctx.lineWidth = (o.lw || 1) * R.lk; if (o.dash) ctx.setLineDash(o.dash); ctx.stroke(); ctx.setLineDash([]); }
      ctx.shadowBlur = 0;
    };
    R.poly = (pts, o) => {
      o = st(o, {});
      const q = pts.map((p) => R.p(p));
      if (q.some((p) => !p.ok)) return;
      const z = q.reduce((a, p) => a + p.z, 0) / q.length;
      const fz = o.nofog ? 1 : R.fog(z);
      R.add(z + (o.zb || 0), () => {
        ctx.beginPath(); q.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)));
        if (o.close !== false) ctx.closePath();
        ctx.globalAlpha = (o.alpha == null ? 1 : o.alpha) * fz;
        if (o.grad) { const g = o.grad(ctx, q); if (g) o = Object.assign({}, o, { fill: g }); }
        strokeFill(o); ctx.globalAlpha = 1;
      });
    };
    R.path = (pts, o) => R.poly(pts, st(o, { close: false }));
    R.line = (a, b, o) => R.poly([a, b], st(o, { close: false, stroke: R.Wh(0.4) }));
    R.ring = (c, r, axis, seg = 64, a0 = 0, a1 = Math.PI * 2) => {
      const pts = [];
      for (let i = 0; i <= seg; i++) {
        const a = a0 + (a1 - a0) * (i / seg), ca = Math.cos(a) * r, sa = Math.sin(a) * r;
        pts.push(axis === "y" ? [c[0] + ca, c[1], c[2] + sa] : axis === "x" ? [c[0], c[1] + ca, c[2] + sa] : [c[0] + ca, c[1] + sa, c[2]]);
      }
      return pts;
    };
    R.circle = (c, r, axis, o, seg = 64, a0 = 0, a1 = Math.PI * 2) => R.poly(R.ring(c, r, axis, seg, a0, a1), st(o, { close: a1 - a0 >= Math.PI * 2 - 1e-6 }));

    // face ombrée : alpha du remplissage selon l'éclairage, faces arrière atténuées
    const shadeFace = (P, o, ctr) => {
      const n = norm(cross(sub(P[1], P[0]), sub(P[2], P[0])));
      const fc = mul(P.reduce((a, p) => add(a, p), [0, 0, 0]), 1 / P.length);
      let nn = n; if (ctr && dot(n, sub(fc, ctr)) < 0) nn = mul(n, -1);
      const facing = dot(nn, sub(R.eye, fc)) > 0;
      const li = 0.18 + 0.82 * Math.max(0, dot(nn, R.LIGHT));
      const base = o.fa == null ? 0.05 : o.fa, col = o.tint || "#ffffff", K = T.depthK || 1.6;
      const a1 = base * li * (facing ? 1 : 0.35) * K, a0 = a1 * 0.3;
      let top = P[0], bot = P[0];
      P.forEach((p) => { if (p[1] > top[1]) top = p; if (p[1] < bot[1]) bot = p; });
      const solid = o.fill !== false && a1 > 0;
      R.poly(P, Object.assign({}, o, {
        fill: solid ? DR.rgba(col, a1) : null,
        // dégradé vertical : la lumière tombe d'en haut, le pied de la face s'assombrit
        grad: solid && facing && top[1] - bot[1] > 0.05 ? () => { const pt = R.p(top), pb = R.p(bot), g = ctx.createLinearGradient(pt.x, pt.y, pb.x, pb.y); g.addColorStop(0, DR.rgba(col, a1 * 1.35)); g.addColorStop(1, DR.rgba(col, a0)); return g; } : null,
        stroke: o.stroke ? o.stroke : null, lw: (o.lw || 1) * (facing ? 1.1 : 0.6),
        alpha: (o.alpha == null ? 1 : o.alpha) * (facing ? 1 : 0.4),
      }));
    };
    R.box = (c, s, o) => {
      o = st(o, { fill: R.Wh(0.025), stroke: R.Wh(0.28), lw: 1 });
      const [x, y, z] = c, [w, h, d] = [s[0] / 2, s[1] / 2, s[2] / 2];
      const V = [[x - w, y - h, z - d], [x + w, y - h, z - d], [x + w, y + h, z - d], [x - w, y + h, z - d], [x - w, y - h, z + d], [x + w, y - h, z + d], [x + w, y + h, z + d], [x - w, y + h, z + d]];
      const F = [[0, 1, 2, 3], [4, 5, 6, 7], [0, 1, 5, 4], [3, 2, 6, 7], [0, 3, 7, 4], [1, 2, 6, 5]];
      const vol = o.shade && T.volume;
      if (vol === "slices") { // volume en tranches horizontales (hologramme)
        const n = Math.max(2, Math.round(s[1] / 0.2)), col = o.stroke || R.Wh(0.4), fa = (o.fa == null ? 0.05 : o.fa) * 0.5;
        for (let i = 0; i <= n; i++) { const yy = y - h + (2 * h * i) / n, e = i === 0 || i === n; R.poly([[x - w, yy, z - d], [x + w, yy, z - d], [x + w, yy, z + d], [x - w, yy, z + d]], { stroke: col, lw: (o.lw || 1) * (e ? 1.3 : 0.7), fill: fa > 0 ? DR.rgba(o.tint || "#ffffff", fa) : null, alpha: (o.alpha == null ? 1 : o.alpha) * (e ? 1 : 0.7) }); }
        return;
      }
      if (vol === "points") { // volume en nuage de points
        const stp = Math.max(0.2, Math.max(s[0], s[1], s[2]) / 12), col = o.stroke || R.Wh(0.5);
        const nx = Math.max(1, Math.round(s[0] / stp)), ny = Math.max(1, Math.round(s[1] / stp)), nz = Math.max(1, Math.round(s[2] / stp));
        for (let i = 0; i <= nx; i++) for (let j = 0; j <= ny; j++) for (let k = 0; k <= nz; k++) {
          if (i && i < nx && j && j < ny && k && k < nz) continue;
          const edge = (i === 0 || i === nx ? 1 : 0) + (j === 0 || j === ny ? 1 : 0) + (k === 0 || k === nz ? 1 : 0) >= 2;
          R.dot([x - w + (s[0] * i) / nx, y - h + (s[1] * j) / ny, z - d + (s[2] * k) / nz], edge ? 1.7 : 1.1, { px: true, fill: col, alpha: (o.alpha == null ? 1 : o.alpha) * (edge ? 1 : 0.55) });
        }
        return;
      }
      if (o.shade) F.forEach((f) => shadeFace(f.map((i) => V[i]), o, c));
      else F.forEach((f) => R.poly(f.map((i) => V[i]), o));
    };
    // prisme vertical à n côtés (n grand = cylindre)
    R.prism = (c, r, h, n, o, rot = 0) => {
      o = st(o, { stroke: R.Wh(0.3), lw: 1, shade: true });
      const top = [], bot = [];
      for (let i = 0; i < n; i++) { const a = rot + (i / n) * Math.PI * 2; top.push([c[0] + Math.cos(a) * r, c[1] + h / 2, c[2] + Math.sin(a) * r]); bot.push([c[0] + Math.cos(a) * r, c[1] - h / 2, c[2] + Math.sin(a) * r]); }
      if (T.volume === "slices" || T.volume === "points") {
        const m = Math.max(2, Math.round(h / 0.2)), col = o.stroke || R.Wh(0.4), fa = (o.fa == null ? 0.05 : o.fa) * 0.5, al = o.alpha == null ? 1 : o.alpha;
        for (let j = 0; j <= m; j++) {
          const yy = c[1] - h / 2 + (h * j) / m, ring = [], nn = n > 12 ? 30 : n * 4, e = j === 0 || j === m;
          for (let i = 0; i < nn; i++) {
            if (n > 12) { const a = rot + (i / nn) * Math.PI * 2; ring.push([c[0] + Math.cos(a) * r, yy, c[2] + Math.sin(a) * r]); }
            else { const q = Math.floor(i / 4), f = (i % 4) / 4, A = bot[q], B = bot[(q + 1) % n]; ring.push([A[0] + (B[0] - A[0]) * f, yy, A[2] + (B[2] - A[2]) * f]); }
          }
          if (T.volume === "slices") R.poly(ring, { stroke: col, lw: (o.lw || 1) * (e ? 1.3 : 0.7), fill: fa > 0 ? DR.rgba(o.tint || "#ffffff", fa) : null, alpha: al * (e ? 1 : 0.7) });
          else ring.forEach((p) => R.dot(p, e ? 1.7 : 1.1, { px: true, fill: col, alpha: al * (e ? 1 : 0.55) }));
        }
        return;
      }
      const so = n > 12 ? Object.assign({}, o, { stroke: null }) : o;
      for (let i = 0; i < n; i++) shadeFace([bot[i], bot[(i + 1) % n], top[(i + 1) % n], top[i]], so, c);
      const fa = o.fa == null ? 0.05 : o.fa;
      R.poly(top, { fill: DR.rgba(o.tint || "#ffffff", fa * 1.2), stroke: o.stroke, lw: o.lw, alpha: o.alpha });
      R.poly(bot, { fill: DR.rgba(o.tint || "#ffffff", fa * 0.6), stroke: o.stroke, lw: o.lw, alpha: o.alpha });
      if (n > 12) {
        // silhouettes du cylindre : deux arêtes verticales tangentes à la vue
        const vr = norm(cross(R.fw, [0, 1, 0]));
        [-1, 1].forEach((sg) => R.line([c[0] + vr[0] * r * sg, c[1] - h / 2, c[2] + vr[2] * r * sg], [c[0] + vr[0] * r * sg, c[1] + h / 2, c[2] + vr[2] * r * sg], { stroke: o.stroke, lw: o.lw, alpha: o.alpha }));
      }
    };
    R.dot = (c, r, o) => {
      o = st(o, { fill: R.A(1) });
      const p = R.p(c);
      if (!p.ok) return;
      const rr = o.px ? r : r * p.k, fz = o.nofog ? 1 : R.fog(p.z);
      R.add(p.z + (o.zb || 0), () => {
        ctx.beginPath(); ctx.arc(p.x, p.y, Math.max(0.5, rr), 0, Math.PI * 2);
        ctx.globalAlpha = (o.alpha == null ? 1 : o.alpha) * fz; strokeFill(o); ctx.globalAlpha = 1;
      });
    };
    // halo sphérique (dégradé radial) — agents, orbes, flaques
    R.halo = (c, r, col, a = 1, o = {}) => {
      const p = R.p(c);
      if (!p.ok) return;
      const rr = r * p.k;
      R.add(p.z + (o.zb || 0), () => {
        const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, rr);
        g.addColorStop(0, DR.rgba(col, a)); g.addColorStop(o.mid || 0.35, DR.rgba(col, a * 0.35)); g.addColorStop(1, DR.rgba(col, 0));
        ctx.fillStyle = g; ctx.beginPath(); ctx.arc(p.x, p.y, rr, 0, Math.PI * 2); ctx.fill();
      });
    };
    // flaque de lumière posée sur le plan y = c[1]
    R.pool = (c, r, col, a = 0.25) => {
      const q = R.ring(c, r, "y", 40).map((v) => R.p(v));
      if (q.some((v) => !v.ok)) return;
      const ctr = R.p(c);
      R.add(ctr.z + 50, () => {
        ctx.save(); ctx.beginPath(); q.forEach((v, i) => (i ? ctx.lineTo(v.x, v.y) : ctx.moveTo(v.x, v.y))); ctx.closePath(); ctx.clip();
        const rr = Math.max(...q.map((v) => Math.hypot(v.x - ctr.x, v.y - ctr.y)));
        const g = ctx.createRadialGradient(ctr.x, ctr.y, 0, ctr.x, ctr.y, rr);
        g.addColorStop(0, DR.rgba(col, a)); g.addColorStop(1, DR.rgba(col, 0));
        ctx.fillStyle = g; ctx.fillRect(ctr.x - rr, ctr.y - rr, rr * 2, rr * 2); ctx.restore();
      });
    };
    R.trail = (pts, col, w = 4, a = 1) => {
      for (let i = 1; i < pts.length; i++) R.line(pts[i - 1], pts[i], { stroke: DR.rgba(col, a * (i / pts.length)), lw: w * (0.3 + 0.7 * (i / pts.length)), zb: -0.5 });
    };
    R.text = (c, str, o) => {
      o = st(o, { color: R.Wh(0.85), align: "center", base: "middle", weight: 500, font: "body" });
      const p = R.p(c);
      if (!p.ok) return;
      let px = (o.px || 13) * (T.textK || 1);
      if (o.size) px = DR.clamp(o.size * p.k, o.min || 10, o.max || 120);
      else if (px < 30) px *= T.labelK || 1; // noms des éléments 3D plus grands ; les grands titres ne bougent pas
      if (R.narrow) px *= 0.88;
      const up = T.upper && o.upper !== false && o.font !== "mono";
      const s = up ? String(str).toUpperCase() : String(str);
      const ls = (o.ls || 0) + (up ? 1.2 : 0);
      R.add(p.z + (o.zb == null ? -0.01 : o.zb), () => {
        ctx.font = `${o.weight} ${R.cond(o.font)}${px}px ${R.font(o.font)}`;
        ctx.textAlign = o.align; ctx.textBaseline = o.base;
        if (ls) ctx.letterSpacing = ls + "px";
        if (o.glow && R.gk > 0) { ctx.shadowColor = o.glowColor || R.A(0.8); ctx.shadowBlur = o.glow * R.gk; }
        ctx.globalAlpha = o.alpha == null ? 1 : o.alpha;
        ctx.fillStyle = o.color;
        if (o.outline) { ctx.strokeStyle = o.color; ctx.lineWidth = o.outline; }
        s.split("\n").forEach((ln, i) => (o.outline ? ctx.strokeText : ctx.fillText).call(ctx, ln, p.x + (o.dx || 0), p.y + (o.dy || 0) + i * px * 1.22));
        ctx.globalAlpha = 1; ctx.shadowBlur = 0; if (ls) ctx.letterSpacing = "0px";
      });
    };
    // texte posé dans l'espace : u = sens d'écriture, v = haut des lettres, size = hauteur monde
    R.text3 = (c, str, o) => {
      o = st(o, { size: 1, u: [1, 0, 0], v: [0, 1, 0], color: R.Wh(0.2), weight: 900, font: "display", align: "center", base: "alphabetic" });
      const p0 = R.p(c), pu = R.p(add(c, o.u)), pv = R.p(add(c, o.v));
      if (!p0.ok || !pu.ok || !pv.ok) return;
      const fz = o.nofog ? 1 : R.fog(p0.z), txt = T.upper ? String(str).toUpperCase() : String(str);
      R.add(p0.z + (o.zb || 0), () => {
        ctx.save();
        ctx.transform(pu.x - p0.x, pu.y - p0.y, p0.x - pv.x, p0.y - pv.y, p0.x, p0.y);
        ctx.scale(o.size / 100, o.size / 100);
        ctx.font = `${o.weight} 100px ${R.font(o.font)}`; ctx.textAlign = o.align; ctx.textBaseline = o.base;
        if (o.ls) ctx.letterSpacing = o.ls + "px";
        ctx.globalAlpha = (o.alpha == null ? 1 : o.alpha) * fz;
        if (o.glow && R.gk > 0) { ctx.shadowColor = o.glowColor || R.A(0.6); ctx.shadowBlur = o.glow * R.gk; }
        if (o.fill !== false) { ctx.fillStyle = o.color; ctx.fillText(txt, 0, 0); }
        ctx.shadowBlur = 0;
        if (o.outline) { ctx.strokeStyle = o.outlineColor || o.color; ctx.lineWidth = o.outline; ctx.strokeText(txt, 0, 0); }
        ctx.restore();
      });
    };
    // ombre portée douce sur le plan y = 0 (lumière en haut à gauche)
    R.shadow = (c, r, h, a = 0.55) => R.pool([c[0] + (-R.LIGHT[0] / R.LIGHT[1]) * h * 0.6, 0.012, c[2] + (-R.LIGHT[2] / R.LIGHT[1]) * h * 0.6], r, "#000000", a);
    R.raw = (z, fn) => R.add(z, () => fn(ctx));
    R.flush = () => { R.list.sort((a, b) => b.z - a.z); for (const it of R.list) it.fn(); R.list.length = 0; };
    return R;
  };
})();
