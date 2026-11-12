/* dry-run · thème de la scène : « Laque noire » (design V8.2, 2026-11-12).
   Deux couleurs seulement : noir pur + menthe. Tout le reste est du blanc en opacité.
   Pour changer d'identité, on ne modifie que ce fichier. */
(function () {
  "use strict";
  window.DR = window.DR || {};
  DR.THEME = {
    id: "reflet",
    name: "Laque noire",
    accent: "#3DFFB0",          // menthe
    bg: "#000000",              // noir pur
    bgFx: "spot",               // projecteur venu du haut
    plane: "none",              // sol nu : un miroir laqué, sans motif
    sheen: 2.6, horizon: 1.8,   // reflet du sol et lueur d'horizon
    particles: "none",
    glowK: 1.0, bloom: 0.35,    // halo lumineux, discret : le noir doit rester profond
    lineK: 0.8, glassA: 0.09,   // traits fins, verre léger
    vault: "monolith", agent: "ring", orb: "pulse",
    ghost: true,                // bouton en contour
    words: "poles", wordStyle: "outline", wordWeight: 200, wordLs: 0.3, // « réel » en haut, « simulé » en bas, à plat
    verdictWeight: 200, verdictLs: 26, verdictPx: 110,
    capWeight: 300, brandWeight: 300,
    camera: { pitch: 0.08, fov: 1.12, dist: 20, target: [-1.2, 0.3, 0] }, // vue basse, grand angle
    idea: "au-dessus le réel · en dessous le bac à sable",
    hello: "le reflet décide. <em>le réel suit.</em>",
  };
})();
