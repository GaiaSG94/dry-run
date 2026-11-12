/* dry-run · planches V9 — calque d'étiquettes à plat, sans chevauchement.
   Toute écriture de la scène passe par R.label(point3D, texte, options) :
   l'étiquette est posée à l'écran (jamais en perspective), sur une plaque,
   PAR-DESSUS la 3D (aucun volume ne la traverse). Un placement glouton par
   priorité écarte les étiquettes qui se chevaucheraient ; si une étiquette a
   dû s'éloigner de son point d'ancrage, un trait fin la relie à son élément. */
(function () {
  "use strict";
  const DR = (window.DR = window.DR || {});

  DR.withLabels = function (R) {
    R.L = [];
    R.label = (p3, text, o) => {
      if (text == null || text === "") return;
      const p = R.p(p3);
      if (!p.ok) return;
      R.L.push({ x: p.x, y: p.y, text: String(text), o: Object.assign({ pr: 5, side: "below", size: 14, weight: 700, plate: "dark", alpha: 1, gap: 8 }, o || {}) });
    };
    const rr = (ctx, x, y, w, h, r) => { ctx.beginPath(); ctx.moveTo(x + r, y); ctx.arcTo(x + w, y, x + w, y + h, r); ctx.arcTo(x + w, y + h, x, y + h, r); ctx.arcTo(x, y + h, x, y, r); ctx.arcTo(x, y, x + w, y, r); ctx.closePath(); };
    const hit = (a, b, m) => a.x < b.x + b.w + m && b.x < a.x + a.w + m && a.y < b.y + b.h + m && b.y < a.y + a.h + m;

    R.solveLabels = function (keepOut) {
      const ctx = R.ctx, A = R.A, placed = (keepOut || []).slice();
      const minX = R.padL + 6, maxX = R.W - R.padR - 6, minY = R.padT + 4, maxY = R.H - 6;
      const list = R.L.sort((a, b) => a.o.pr - b.o.pr);
      for (const L of list) {
        const o = L.o, k = R.narrow ? 0.86 : 1, fs = o.size * k, ss = (o.subSize || Math.max(11, o.size * 0.72)) * k;
        const fam = o.mono ? DR.MONO : DR.SANS;
        ctx.font = `${o.weight} ${fs}px ${fam}`;
        if (o.ls) ctx.letterSpacing = o.ls + "px";
        let w = ctx.measureText(L.text).width;
        ctx.letterSpacing = "0px";
        let sw = 0;
        if (o.sub) { ctx.font = `500 ${ss}px ${o.subMono === false ? DR.SANS : DR.MONO}`; sw = ctx.measureText(o.sub).width; }
        const num = o.num != null ? fs * 1.35 : 0;
        const px = o.plate === "none" ? 2 : 10, py = o.plate === "none" ? 0 : 6;
        const W = Math.max(w + num, sw) + px * 2, H = fs * 1.15 + (o.sub ? ss * 1.35 : 0) + py * 2;
        const g = o.gap;
        const pref = {
          below: [L.x - W / 2, L.y + g], above: [L.x - W / 2, L.y - g - H],
          right: [L.x + g, L.y - H / 2], left: [L.x - g - W, L.y - H / 2], center: [L.x - W / 2, L.y - H / 2],
        };
        const order = [o.side].concat(o.fixed ? [] : ["below", "above", "right", "left"].filter((s) => s !== o.side));
        const steps = o.fixed ? [0] : [0, 1, -1, 2, -2, 3, -3, 4, 5];
        let best = null;
        outer: for (const sd of order) {
          for (const st of steps) {
            const horiz = sd === "right" || sd === "left";
            let x = pref[sd][0] + (horiz ? 0 : 0), y = pref[sd][1] + st * (H * 0.55 + 4) * (sd === "above" ? -1 : 1);
            if (horiz) y = pref[sd][1] + st * (H + 4);
            x = Math.max(minX, Math.min(maxX - W, x)); y = Math.max(minY, Math.min(maxY - H, y));
            const r = { x, y, w: W, h: H, sd };
            if (!placed.some((q) => hit(r, q, 3))) { best = r; break outer; }
            if (!best) best = Object.assign({ fallback: true }, r);
          }
        }
        placed.push(best);
        L.r = best; L.W = W; L.H = H; L.fs = fs; L.ss = ss; L.fam = fam; L.px = px; L.py = py; L.num = num;
        const p0 = pref[o.side];
        L.moved = Math.hypot(best.x - Math.max(minX, Math.min(maxX - W, p0[0])), best.y - Math.max(minY, Math.min(maxY - H, p0[1]))) > 10;
      }
      // dessin : traits de rappel d'abord, plaques ensuite
      ctx.save(); ctx.setTransform(R.dpr, 0, 0, R.dpr, 0, 0);
      for (const L of list) {
        if (!L.moved || L.o.noLead) continue;
        const r = L.r, cx = Math.max(r.x, Math.min(r.x + r.w, L.x)), cy = Math.max(r.y, Math.min(r.y + r.h, L.y));
        ctx.globalAlpha = 0.55 * L.o.alpha; ctx.strokeStyle = "rgba(255,255,255,.7)"; ctx.lineWidth = 1;
        ctx.beginPath(); ctx.moveTo(L.x, L.y); ctx.lineTo(cx, cy); ctx.stroke();
        ctx.beginPath(); ctx.arc(L.x, L.y, 2.2, 0, Math.PI * 2); ctx.fillStyle = "#fff"; ctx.fill();
      }
      for (const L of list) {
        const o = L.o, r = L.r;
        ctx.globalAlpha = o.alpha;
        let fg = o.color || "#fff", sub = o.subColor || "rgba(255,255,255,.62)";
        if (o.plate === "dark") { rr(ctx, r.x, r.y, r.w, r.h, 8); ctx.fillStyle = "rgba(4,8,7,.78)"; ctx.fill(); ctx.strokeStyle = o.border || "rgba(255,255,255,.16)"; ctx.lineWidth = 1; ctx.stroke(); }
        if (o.plate === "accent") { rr(ctx, r.x, r.y, r.w, r.h, 8); ctx.fillStyle = A(1); ctx.shadowColor = A(0.7); ctx.shadowBlur = 18; ctx.fill(); ctx.shadowBlur = 0; fg = "#00170d"; sub = "rgba(0,23,13,.72)"; }
        if (o.plate === "white") { rr(ctx, r.x, r.y, r.w, r.h, 8); ctx.fillStyle = "#fff"; ctx.fill(); fg = o.color || "#05301f"; sub = "rgba(5,48,31,.7)"; }
        let tx = r.x + L.px;
        if (o.num != null) {
          const d = L.fs * 1.05, cx = tx + d / 2, cy = r.y + L.py + L.fs * 0.58;
          ctx.beginPath(); ctx.arc(cx, cy, d / 2, 0, Math.PI * 2);
          ctx.fillStyle = o.plate === "accent" ? "#00170d" : A(0.95); ctx.fill();
          ctx.fillStyle = o.plate === "accent" ? A(1) : "#00170d"; ctx.font = `800 ${L.fs * 0.68}px ${DR.SANS}`; ctx.textAlign = "center"; ctx.textBaseline = "middle";
          ctx.fillText(String(o.num), cx, cy + 0.5);
          tx += L.num;
        }
        ctx.font = `${o.weight} ${L.fs}px ${L.fam}`; ctx.textAlign = "left"; ctx.textBaseline = "top";
        if (o.ls) ctx.letterSpacing = o.ls + "px";
        if (o.glow) { ctx.shadowColor = o.glowColor || A(0.8); ctx.shadowBlur = o.glow; }
        if (o.outline) { ctx.strokeStyle = fg; ctx.lineWidth = o.outline; ctx.strokeText(L.text, tx, r.y + L.py + 1); }
        else { ctx.fillStyle = fg; ctx.fillText(L.text, tx, r.y + L.py + 1); }
        ctx.shadowBlur = 0; ctx.letterSpacing = "0px";
        if (o.sub) { ctx.font = `500 ${L.ss}px ${o.subMono === false ? DR.SANS : DR.MONO}`; ctx.fillStyle = sub; ctx.fillText(o.sub, r.x + L.px, r.y + L.py + L.fs * 1.18); }
      }
      ctx.restore(); ctx.globalAlpha = 1;
      R.L.length = 0;
    };
    return R;
  };
})();
