/* dry-run · agencement « Plateau » (design V9.C) : vue de dessus, la demande suit une route
   au sol en serpentin. Coordonnées : x = gauche → droite, y = hauteur (sol = 0), z = vers l'écran. */
(function () {
  "use strict";
  const DR = window.DR;
  /* C · PLATEAU — vue de dessus, comme un plateau de jeu : la demande suit une route
     au sol en serpentin (rangée du fond : demande → règles ; rangée de devant :
     verdict → coffre → chaîne → destinataire). Rien n'est caché derrière rien. */
  const C = {
    id: "plateau", nav: { next: "README.md", nextL: "aide" },
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

  DR.LAYOUT = C;
})();
