/* dry-run · thème de la scène : « Laque noire », version Plateau (design V9, 2026-11-12).
   Deux couleurs seulement : noir pur + menthe. Terminal en verre poli.
   Pour changer d'identité, on ne modifie que ce fichier. */
(function () {
  "use strict";
  window.DR = window.DR || {};
  DR.THEME = {
    id: "plateau", accent: "#3DFFB0", bg: "#000000", sheen: 2.2, glassA: 0.09, bloom: 0.25, lineK: 1.9, glowK: 1, labelK: 1, drift: 0.025,
    glass: "poli",
    // l'unique étiquette, au repos : ce que fait Dry Run
    pitch: "<b>Dry Run</b> protège le portefeuille d’un agent IA sur NEAR : chaque transaction est d’abord jouée dans une copie de la chaîne, et n’est signée que si ses effets réels sont sûrs.",
  };
})();
