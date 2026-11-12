# Prompt opencode — déposer « Laque noire » (V8.2) dans `web/`

> À coller tel quel dans un chat opencode. Les fichiers sont déjà écrits et
> vérifiés en mode DÉMO : il s'agit d'un dépôt, pas d'une réécriture.
> Remplace `PROMPT-OPENCODE-V7.md` (périmé).

---

```
Projet : dry-run (hackathon NEAR, track AI Security & Defense).
Racine : HACKATONS/2026-11-15-near/Security/dry-run/

But : mettre en production l'identité « Laque noire » du design V8.2.
La page complète existe déjà, prête à déposer, dans G-DESIGN/web-laque/.
Ouvre G-DESIGN/web-laque/index.html pour voir le résultat attendu.

1. Copier (écraser) :
     G-DESIGN/web-laque/index.html          → web/index.html
     G-DESIGN/web-laque/scene/renderer.js   → web/scene/renderer.js
     G-DESIGN/web-laque/scene/shell.js      → web/scene/shell.js
     G-DESIGN/web-laque/scene/reflet8.js    → web/scene/reflet8.js   (nouveau)
     G-DESIGN/web-laque/scene/theme.js      → web/scene/theme.js     (nouveau)

2. NE PAS copier data.js, timeline.js, live.js : ceux de G-DESIGN/web-laque/scene/
   sont des copies de web/scene/ faites le 2026-11-12 pour pouvoir tester le
   paquet. Garder les versions de web/scene/. Si elles ont changé depuis,
   vérifier seulement que rien ne casse.

3. Supprimer web/scene/reflet.js si plus rien ne le charge (grep).
   Ne pas toucher classic.html, style.css, app.js.

4. Contrat à vérifier avec live.js (inchangé par rapport à l'ancien shell) :
   ids #cap #bar #scen #play #badge #badge-t #brnote, et
   DR.app.{setT, play, pause, stop, setBadge, setPlayLabel, onScenarios}.
   La scène lit aussi s.certLabel et DR.vaultNow(s).

5. Mettre à jour web/README.md : tableau des fichiers (reflet8.js = la scène,
   theme.js = l'identité : 2 couleurs, noir #000000 + menthe #3DFFB0), et la
   phrase d'intro (la couleur d'accent n'est plus #C6FF3D).

Contraintes inchangées : zéro CDN, zéro dépendance, zéro image, zéro réseau
hors /api/* en mode LIVE ; la page s'ouvre en double-clic (file://) en mode
DÉMO ; montants en BigInt yoctoNEAR ; rien hors de web/ ; rien dans G-DESIGN/.

À savoir :
- shell.js corrige un défaut de l'ancien shell : le pas de temps `dt` pouvait
  être négatif sur une machine lente et projeter la caméra très loin.
- Avec ?t=<secondes> la page se fige à cet instant et la caméra se pose
  directement : pratique pour les captures et la vidéo.

Vérification obligatoire :
a) node --check sur chaque fichier de web/scene/.
b) Mode DÉMO (file://) : comparer web/index.html à G-DESIGN/web-laque/index.html
   aux instants ?t=0, 12.3 (WARN), 23 (certificat), 27 (simulé = réel),
   39.5 (BLOCK), 42 (boucle MCP), 61 (vol de clé), 64 (gel), 70.5 (banc),
   en 1600×900 et 390×844. Aucune erreur console, même rendu.
c) Mode LIVE : cargo run -p dryrun-gate -- serve --port 8787 --sim stub, puis
   http://localhost:8787 : le badge passe en « live », les boutons de scénarios
   proposent au vrai pare-feu, la scène joue la vraie réponse (verdict, règles,
   effets, certificat avec son digest), le gel fonctionne.
d) classic.html s'ouvre toujours.
e) make test et make lint passent (aucun changement Rust attendu).

Rendu : liste des fichiers modifiés, captures aux mêmes instants, et tout
écart avec le paquet (avec la raison).
```
