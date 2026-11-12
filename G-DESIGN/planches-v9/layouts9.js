/* dry-run · planches V9 — thème commun + trois agencements de la scène.
   Coordonnées : x = gauche → droite, y = hauteur (sol = 0), z = profondeur (vers l'écran = +). */
(function () {
  "use strict";
  const DR = window.DR;

  DR.T9 = (glass) => ({
    id: "laque9", accent: "#3DFFB0", bg: "#000000", sheen: 2.2, glassA: 0.09, bloom: 0.25, lineK: 1.9, glowK: 1, labelK: 1, drift: 0.025, glass,
    pitch: "<b>Dry Run</b> protège le portefeuille d’un agent IA sur NEAR : chaque transaction est d’abord jouée dans une copie de la chaîne, et n’est signée que si ses effets réels sont sûrs.",
  });

  /* A · LIGNE — un schéma de gauche à droite, 7 étapes numérotées sur une même dalle.
     Le bac à sable est une boîte de verre en pointillés qui contient les copies. */
  const A = {
    id: "ligne", nav: { next: "index.html", nextL: "planches" },
    camera: { yaw: -0.06, pitch: 0.34, dist: 34, fov: 0.9, target: [-1.6, 1.2, 0] }, centerY: 0.5,
    follow: 0, focus: true, floor: [-17, 16, -3.4, 3.4],
    P: {
      agent: [-15, 2.4, 0], gate: [-11, 0, 0], sand: [-5.2, 0, 0], rules: [0.6, 0, 0], verdict: [0.6, 3.1, 0],
      vault: [5.6, 0, 0], cert: [5.6, 4.9, 0], chainA: [8.3, 0, 0], chainB: [11.1, 0, 0], recip: [13.6, 0, 0], bench: [26, 0, 0],
    },
    sandW: 6.4, sandD: 3.2, sandH: 3.4, twinV: [-7.3, 1.1, 0], twinR: [-3.1, 0.9, 0],
    stations: [-6.2, -5.7, -5.2, -4.7, -4.2].map((x) => [x, 0, 1.15]), stationTiles: true, sandIn: [-8.4, 0.5, 1.15],
    ruleCols: 6, ruleGap: 0.5, verdictSlot: [0.6, 2.4, 0],
    actorR: [2.8, 5.2, 0], actorV: [8.8, 5.2, 0], via: { cert: [[3.2, 4.4, 0]] }, mcpUp: 3.2,
  };

  /* B · MIROIR RANGÉ — on garde l'idée « réel en haut, sand en bas », mais chaque
     chose a sa colonne : le coffre au-dessus de sa copie, le destinataire au-dessus
     de la sienne, les règles et le verdict à droite, hors du reflet. */
  const B = {
    id: "miroir", nav: { next: "index.html", nextL: "planches" }, mirror: true,
    camera: { yaw: -0.06, pitch: 0.12, dist: 31, fov: 0.95, target: [-1.6, -0.3, 0] }, centerY: 0.5,
    follow: 0, focus: true, stepSide: "above", floor: [-16, 14, -3, 3],
    P: {
      agent: [-14, 2.4, 0], gate: [-10.5, 0, 0], vault: [-6, 0, 0], cert: [-6, 4.9, 0], chainA: [-3.4, 0, 0], chainB: [2.4, 0, 0],
      recip: [5, 0, 0], rules: [10, 0, 0], verdict: [10, 3.1, 0], sand: [-0.5, 0, 0], bench: [24, 0, 0],
    },
    twinSize: [1.8, 3.4, 1.8], twinV: [-6, -1.7, 0], twinR: [5, -1.3, 0], twinK: 1,
    stations: [-3.2, -1.85, -0.5, 0.85, 2.2].map((x) => [x, -1.45, 0]), sandIn: [-3.9, -1.45, 0.6], sandLabelAt: [-0.5, -4.6, 0],
    ruleCols: 6, ruleGap: 0.5, verdictSlot: [10, 2.4, 0],
    via: { send: [[-8.6, -0.7, 1.8], [-4.4, -0.7, 1.8]], rules: [[3.4, -0.6, 2.0], [8.2, -0.3, 2.0]], cert: [[5, 5.4, 0]] },
    actorR: [12.6, 5.0, 0], actorV: [-9.0, 5.2, 0], arcUp: 1.2, mcpUp: 3.6,
  };

  /* C · PLATEAU — vue de dessus, comme un plateau de jeu : la demande suit une route
     au sol en serpentin (rangée du fond : demande → règles ; rangée de devant :
     verdict → coffre → chaîne → destinataire). Rien n'est caché derrière rien. */
  const C = {
    id: "plateau", nav: { next: "index.html", nextL: "planches" },
    camera: { yaw: 0, pitch: 0.98, dist: 22.5, fov: 0.95, target: [-1.2, 0, 0.4] }, centerY: 0.5,
    follow: 0.15, focus: true, floor: [-13.5, 10, -6.4, 6.4],
    P: {
      agent: [-11, 1.6, -3.6], gate: [-7.6, 0, -3.6], sand: [-1.6, 0, -3.6], rules: [5.4, 0, -3.6], verdict: [5.4, 0.4, -6.4],
      vault: [4.6, 0, 3.6], cert: [4.6, 4.2, 3.6], chainA: [2.0, 0, 3.6], chainB: [-3.4, 0, 3.6], recip: [-7, 0, 3.6], bench: [21, 0, 0],
    },
    gateH: 3.2, gateD: 2.4, sandW: 6, sandD: 3, sandH: 2.6, twinV: [-3.5, 1.1, -3.9], twinR: [0.1, 0.9, -3.9],
    stations: [-2.6, -2.1, -1.6, -1.1, -0.6].map((x) => [x, 0, -2.5]), stationTiles: true, sandIn: [-4.6, 0.4, -2.5],
    ruleCols: 4, ruleGap: 0.6, verdictSlot: [5.4, 0.4, -6.0],
    floorPath: [[-11, 0, -1.2], [-7.6, 0, -1.2], [1.8, 0, -1.2], [8.4, 0, -1.2], [8.4, 0, 1.2], [-7, 0, 1.2]],
    via: { rules: [[3.6, 0.6, -1.4]], cert: [[8.2, 2.4, 0]] },
    actorR: [8.6, 3.2, -3.6], actorV: [8.2, 3.2, 3.6], arcUp: 1.0, realArcUp: 1.4, mcpUp: 2.4, benchDist: 14,
  };

  DR.LAYOUTS9 = { A, B, C };
})();
