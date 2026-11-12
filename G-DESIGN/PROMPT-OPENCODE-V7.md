> **PÉRIMÉ (2026-11-11) — NE PAS COLLER.** Gaia a finalement préféré la Menthe néon,
> puis demandé 10 nouvelles planches (V8, `G-DESIGN/planches-v8/index.html`).
> Un nouveau prompt sera écrit quand la planche V8 sera choisie.

# Prompt opencode — implémenter le design V7.1 « Chêne » dans `web/`

> À coller tel quel dans un chat opencode. Identité retenue par Gaia le 2026-11-11 :
> **Chêne** (`chene`, planche `G-DESIGN/planches-v7/03-chene.html`), en version
> V7.1 (3D renforcée, « réel » écrit en grand en haut, « simulé » en bas).

---

```
Projet : dry-run (hackathon NEAR, track AI Security & Defense).
Racine : HACKATONS/2026-11-15-near/Security/dry-run/

But : porter le design V7 « Le Reflet » (identité chene) dans la page web
servie par dryrun-gate, sans rien casser du mode LIVE ni du repli classic.html.

Source du design (ne PAS modifier ces fichiers, les copier) :
  G-DESIGN/planches-v7/renderer7.js   moteur 3D canvas (sur-ensemble compatible de web/scene/renderer.js)
  G-DESIGN/planches-v7/shell7.js      montage HUD + lecteur (mêmes ids et même DR.app que web/scene/shell.js)
  G-DESIGN/planches-v7/reflet7.js     scène « Le Reflet » pilotée par un thème : DR.reflet7(T)
  G-DESIGN/planches-v7/themes.js      les 10 thèmes ; on ne garde que chene
  G-DESIGN/planches-v7/03-chene.html  la planche de référence à reproduire à l'identique

Cible actuelle (à faire évoluer) :
  web/index.html, web/scene/{data,timeline,renderer,shell,reflet,live}.js, web/classic.html

Travail demandé :
1. web/scene/renderer.js ← contenu de renderer7.js (garder le nom renderer.js).
2. web/scene/shell.js ← contenu de shell7.js (garder le nom shell.js). Vérifier que
   live.js fonctionne toujours : #cap, #bar, #scen, #play, #badge, #badge-t,
   #brnote et DR.app.{setT, play, pause, stop, setBadge, setPlayLabel, onScenarios}.
   En production, la navigation en haut à droite reste « tableau de bord »
   (classic.html) et « aide » (README.md) : c'est le défaut de shell7.js quand le
   thème n'a pas de `nav` → supprimer `nav` du thème copié.
3. Créer web/scene/reflet7.js (copie de reflet7.js) et web/scene/theme.js qui
   définit UNIQUEMENT le thème retenu : DR.THEME = { ...l'objet chene de themes.js,
   id: "reflet" } (sans `nav`, sans `swatch`).
4. web/index.html : remplacer <script src="scene/reflet.js"> par
     <script src="scene/reflet7.js"></script>
     <script src="scene/theme.js"></script>
     <script>DR.reflet7(DR.THEME);</script>
   AVANT <script src="scene/live.js">. Ordre final : data, timeline, renderer, shell,
   reflet7, theme, appel DR.reflet7, live. Garder le <noscript> vers classic.html.
5. Supprimer web/scene/reflet.js seulement si plus rien ne le charge (grep).
6. Mettre à jour web/README.md (tableau des fichiers : reflet7.js, theme.js) et
   noter l'identité retenue.

Contraintes (non négociables) :
- Zéro CDN, zéro dépendance, zéro image, zéro réseau sauf /api/* en mode LIVE.
- La page doit toujours s'ouvrir en double-clic (file://) en mode DÉMO.
- Montants en BigInt yoctoNEAR (déjà le cas dans data.js et live.js : ne pas toucher).
- Ne pas modifier data.js, timeline.js, live.js sauf bug bloquant (le documenter).
- Ne rien modifier hors de web/. Aucun fichier dans G-DESIGN/.
- 2 couleurs seulement (fond + accent du thème), le reste en encre du thème.

Vérification obligatoire :
a) node --check sur chaque fichier de web/scene/.
b) Mode DÉMO : ouvrir web/index.html en file:// puis comparer à la planche de
   référence G-DESIGN/planches-v7/03-chene.html aux instants
   ?t=0, ?t=12.3 (WARN), ?t=23 (certificat), ?t=27 (simulé = réel),
   ?t=39.5 (BLOCK), ?t=61 (vol de clé), ?t=70.5 (banc). Captures headless à
   1600×900 et 390×844 : aucune erreur console, rien de coupé, même rendu.
c) Mode LIVE : cargo run -p dryrun-gate -- serve --port 8787 --sim stub, puis
   ouvrir http://localhost:8787 : le badge passe en « live », les boutons de
   scénarios proposent au vrai pare-feu, la scène joue la vraie réponse
   (verdict, règles, effets, certificat avec son digest), le gel fonctionne.
d) classic.html s'ouvre toujours.
e) make test et make lint passent (aucun changement Rust attendu).

Rendu final : la liste des fichiers modifiés, les captures avant/après aux
mêmes instants, et tout écart avec la planche de référence (avec la raison).
```

---

## Rappel des clés et des fichiers

| Clé | Fichier de référence | Accent | Fond |
|---|---|---|---|
| acide | `01-acide.html` | `#C6FF3D` | `#050807` |
| elite | `02-elite.html` | `#19C27A` | `#020D07` |
| chene | `03-chene.html` | `#8DA34A` | `#0B0C07` |
| sauge | `04-sauge.html` | `#A9C9A4` | `#0B100D` |
| menthe | `05-menthe.html` | `#3DFFB0` | `#010605` |
| jade | `06-jade.html` | `#14E3A6` | `#01100D` |
| sapin | `07-sapin.html` | `#45B866` | `#030805` |
| phosphore | `08-phosphore.html` | `#39FF14` | `#000200` |
| kaki | `09-kaki.html` | `#A7B86A` | `#0A0B07` |
| anglais | `10-anglais.html` | `#EAF8D8` | `#06281A` |
