# DESIGN.md — dry-run (NEAR, AI Security & Defense)

> **AVERTISSEMENT (2026-11-11)** : les versions V1 à V5 décrites ci-dessous sont
> **rejetées** par Gaia. Le design repart de zéro.
> **Le brief qui fait foi est `RESTART-DESIGN.md`.**
> Ce fichier n'est conservé que comme historique des rounds — ne pas le reprendre,
> ne pas l'amender, ne pas « améliorer » une de ces versions.

Direction artistique de l'interface web (`web/`). Repo code : lecture seule.
Ton imposé par le brief initial : sécurité (rigueur, densité maîtrisée) + lisibilité sous stress.
Juges : implémentation, design, impact, originalité. Deadline design : 14/11.

---

## Historique des rounds (tous rejetés)

### V1 — 2026-11-11 · `/planches 6`, six directions textuelles
Poste de contrôle · Rapport d'expertise · Néon pare-feu · Cockpit de coffre-fort ·
Journal des interceptions · Attention tunnel. Tableau de décision + recommandation
hybride 6+4+2.
**Rejeté** : descriptions textuelles, aucune identité visuelle réelle, rien de marquant.

### V2 — 2026-11-11 · planches HTML cliquables (`planches/`)
Les six directions rendues en HTML autonome : `base.css` + `theme-1..6.css`,
contenu commun dans `data.js`, rendu dans `planche.js`. Contenu complet (coffre,
formulaire + intention, carte des effets, certificat, frise, banc 200 robots,
R1–R12, défense on-chain, parcours, surfaces API/CLI/MCP), 7 scénarios cliquables,
gel, responsive vérifié 1600/1180/390 px.
**Rejeté** : tableaux de bord documentés. Trop de texte, aucune identité,
« un bout visuel et le reste en texte ».

### V3 — 2026-11-11 · trois directions fortes (`planches-v3/`)
SIGNAL (noir + rouge `#FF3B14`), ORBIT (nuit + lime `#C9F73A`), STRIP (nuit +
glace `#7CE7FF`). Deux couleurs par direction, fond sombre, un bouton qui joue
tout le processus, moteur commun `core.js`.
**Rejeté** : meilleure identité, mais toujours des panneaux 2D alignés dans une
grille avec une petite animation au milieu. Pas un espace.

### V4 — 2026-11-11 · PIPELINE (`planches-v3/pipeline.html`)
Le bon récit : agent A → motif de la transaction → bac à sable et ses 5
sous-étapes → effets → règles → verdict → certificat → chaîne → agent B, avec
boucle de reformulation côté agent (MCP) et compteur d'essais. Scénario
« hors plafond » ajouté pour démontrer la boucle.
**Rejeté** : le récit était juste, la forme restait des colonnes de texte.

### V5 — 2026-11-11 · dix mondes 3D (`planches-3d/`)
Moteur 3D partagé (CSS 3D `preserve-3d` + canvas 2D pour étoiles, grille,
anneaux, liaisons, paquet lumineux), dix configurations d'espace (Orbite,
Tunnel, Strates, Grille, Constellation, Cube, Hélice, Skyline, Table, Atome),
13 panneaux de verre, caméra orbite/zoom/focus, déroulé complet en un bouton.
**Rejeté** : 13 panneaux flottants trop petits, scène encombrée, chevauchements,
texte illisible à l'échelle, aucune hiérarchie, rien de compréhensible d'un coup
d'œil. Dix variantes d'une même mauvaise idée au lieu d'une idée forte.

---

## Leçon transmise au round suivant

La 3D n'est pas une décoration à ajouter sur la même maquette. Il faut d'abord
choisir **une métaphore spatiale qui raconte le produit** (sas, poste de
contrôle, tunnel de vérification, balance, couloir de sécurité…), la pousser à
fond, et n'y mettre que ce qui la sert. Moins d'éléments, plus grands, plus
lisibles, plus assumés. Une seule idée forte par piste.

Le contenu à rendre visible (inventaire fonctionnel complet, vérifié dans le
code) est dans `RESTART-DESIGN.md` §3. Les règles de forme sont dans
`RESTART-DESIGN.md` §2. La liste des pièges déjà rencontrés est dans
`RESTART-DESIGN.md` §6.

---

## V6 — 2026-11-11 · dix métaphores spatiales (`planches-v6/`) — EN REVUE

Demande de Gaia : « 10 planches de design très différentes » (cela remplace les
3 pistes prévues par `RESTART-DESIGN.md` §5). Point d'entrée : `planches-v6/index.html`.

**Principe** : une idée spatiale forte par planche, et un espace 3D entier par
planche. `core.js` ne dessine rien : il fournit seulement les données réelles,
la chronologie du bouton unique et une projection 3D en canvas (zéro dépendance,
montants en BigInt yocto). Chaque planche invente sa scène, sans réutiliser une
scène commune comme en V5.

**Le même déroulé réel dans les 10 planches** (un bouton, ≈ 72 s) :
1. swap good-dex : premier contact WARN R5 → `approve_contract` 1 yocto →
   re-simulation → ALLOW → certificat (7 vérifications) → exécution →
   simulé == réel ; R12 différentiel 980 ‰ / 975 ‰ (make demo, étapes 3-4) ;
2. injection « 49 NEAR à mallory » → BLOCK R1, puis la boucle MCP **côté agent**
   (`wallet_explain`, optionnelle, le pare-feu ne reformule jamais) ;
3. drainer (contrat déjà approuvé) → BLOCK R8 : ce sont les effets qui l'arrêtent,
   pas la liste blanche ;
4. vol de clé d'agent → refus on-chain `ERR_OVER_TX_CAP` ;
5. gel d'un geste → FROZEN ;
6. banc de 200 robots, graine 42 (`bench_matrix.json`) : TP 100, TN 100, FP 0,
   FN 0, F1 1,00, jugement 0 µs en médiane et 2 µs au p95, 6 pièges.

**Code couleur** : 2 couleurs par planche (fond + accent, le reste en blanc à
opacité variable). ALLOW, WARN et BLOCK se distinguent par la forme (plein,
contour, fissure ou équivalent), jamais par une 3ᵉ couleur. Chaque planche
affiche son code dans sa légende.

| # | Planche | Accent | Idée |
|---|---|---|---|
| 01 | Le Reflet | `#C6FF3D` | surface de verre : réel au-dessus, bac à sable en reflet dessous |
| 02 | Le Sas | `#7DF9FF` | couloir en profondeur, deux portes, capsule gravée du motif |
| 03 | La Chambre forte | `#FFC247` | cube-coffre central, 12 serrures, double fantôme |
| 04 | Le Prisme | `#A98BFF` | le bac à sable décompose le faisceau en spectre d'effets |
| 05 | La Balance | `#FF4FD8` | annoncé contre observé, aiguille sur cadran R1–R12 |
| 06 | La Cloche | `#3DFFB0` | diorama isométrique + copie sous cloche de verre |
| 07 | Le Scanner | `#FF2D55` | anneau qui tranche la demande en couches d'effets |
| 08 | Le Crash-test | `#FFE600` | chariot lancé contre un mur de capteurs, au ralenti |
| 09 | L'Écluse | `#3D7BFF` | les fonds sont de l'eau ; portes ouvertes si niveaux concordent |
| 10 | L'Essaim | `#FF7A1A` | 200 robots contre une membrane ; on en suit un, puis tous |

**Changelog vs V5** : 10 métaphores au lieu de 10 dispositions des mêmes
13 panneaux · aucun panneau de texte · textes à taille écran fixe (≥ 10 px) ·
le contenu vit dans les objets (niveau de réserve, anneau 24 h, serrures,
fissures…) · scénario réel de la démo (WARN → approbation → ALLOW) au lieu du
scénario « hors plafond » · vérification par captures figées (`?t=`).

**Interaction** : glisser = tourner (borné) · molette = zoom · double-clic =
recentrer · espace = jouer/pause · ← → = ±2 s · `?t=12.3` fige la planche à
un instant donné (utile pour la vidéo et les captures).

**Retour attendu** : Garder / Changer (deltas numérotés) / Essayer, et le choix
d'une ou deux planches à pousser (`/compare A B`, puis `/lock`).

**Issue de V6** : Gaia a retenu **Le Reflet**, qu'opencode a déjà porté dans
`web/` (scène + mode LIVE). Les autres planches V6 sont restées incomplètes
(02, 03, 05, 06 et 08 partielles ; 04, 07, 09 et 10 absentes) : les agents se
sont arrêtés sur la limite d'usage. Elles ne sont pas reprises.

---

## V7 — 2026-11-11 · Le Reflet en 10 identités (`planches-v7/`) — EN REVUE

Demande de Gaia : partir de la page actuelle `web/index.html` (Le Reflet), la
rendre vraiment moderne, travailler beaucoup plus la 3D et les animations,
réduire le texte et grossir l'essentiel, proposer 10 identités plus marquées et
plusieurs verts, dont des verts plus foncés. Point d'entrée : `planches-v7/index.html`.

**Base technique** : `base/data.js` et `base/timeline.js` sont des copies
inchangées de `web/scene/`. `renderer7.js` est un sur-ensemble compatible de
`renderer.js`. `shell7.js` garde le même contrat que `shell.js` (ids, `DR.app`) :
`web/scene/live.js` tourne dessus sans erreur, vérifié. `reflet7.js` est la scène,
pilotée par un thème, et `themes.js` contient les 10 thèmes.

**Changements communs (vs `web/` actuel)** :
- 3D : volumes de verre ombrés selon une lumière, faces arrière atténuées,
  brume de profondeur, flaques de lumière sous les objets, caméra qui dérive
  lentement et cadrages par phase plus marqués, pare-feu en dégradé avec un
  reflet qui glisse ;
- animations : traînée de la demande en vol, onde de choc du verdict sur le
  plan (pleine pour ALLOW, pointillée pour WARN, blanche pour BLOCK), blocs de
  chaîne qui s'allument un à un, anneau qui tourne autour de la sous-étape en
  cours, sceau du certificat qui tourne, agent qui pivote, particules d'ambiance ;
- texte : supprimés « vault.test.near », « journal 100 · n », « destinataire ·
  pas un agent » (le type reste affiché), les numéros d'étapes et la note
  R12 longue. Agrandis : solde (24 px), verdict (88 à 120 px), « simulé = réel »
  (40 px), montants (19 px), règle qui mord (16 px), sous-étape en cours (17 px),
  légende du bas (21 px), F1 (30 px). Les libellés des deux mondes sont ancrés
  au bord droit, sur la ligne du plan.

| # | Identité | Accent | Fond | Signature |
|---|---|---|---|---|
| 01 | Acide | `#C6FF3D` | `#050807` | la version actuelle, poussée : grille, poussière, comète |
| 02 | Élite | `#19C27A` | `#020D07` | serif, rayures fines, coffre-cylindre, verdict fin |
| 03 | Chêne | `#8DA34A` | `#0B0C07` | encre blanc chaud, cernes de bois au sol, spores, verre mat |
| 04 | Sauge | `#A9C9A4` | `#0B100D` | minimal : sol nu, monolithe, traits fins, verdict ultra-léger |
| 05 | Menthe néon | `#3DFFB0` | `#010605` | majuscules, grille néon balayée, pluie, trame CRT |
| 06 | Jade liquide | `#14E3A6` | `#01100D` | le plan devient de l'eau : ondes, plongeon de la demande |
| 07 | Sapin | `#45B866` | `#030805` | sol en points, spores, prisme hexagonal, nocturne |
| 08 | Phosphore | `#39FF14` | `#000200` | terminal : tout en mono, fil de fer, CRT, grain |
| 09 | Kaki tactique | `#A7B86A` | `#0A0B07` | radar au sol, réticules sur les cibles, vue plus haute |
| 10 | Vert anglais | `#EAF8D8` | `#06281A` | le vert devient le fond, accent crème, monolithes |

« Vert éléphant » ne correspond à aucun vert standard que je connaisse. Je l'ai
lu comme « élégant », d'où la Sauge. Les verts plus foncés demandés sont
l'Élite, le Chêne, le Sapin, le Kaki et le Vert anglais en fond.

**Implémentation** : `G-DESIGN/PROMPT-OPENCODE-V7.md`, à coller dans opencode
après avoir choisi la clé de l'identité. Claude ne touche pas `web/`.

**Retour attendu** : Garder / Changer (deltas numérotés) / Essayer, et le choix
de l'identité, éventuellement en mélangeant (ex. sol de Jade + couleurs d'Élite).

---

## V7.1 — 2026-11-11 · Chêne retenu, 3D renforcée — VERROUILLÉ POUR IMPLÉMENTATION

Choix de Gaia : **03 Chêne** (`#8DA34A` sur `#0B0C07`, encre blanc chaud
`#F3EEDD`). Ses retours : la 3D n'est pas assez poussée ni assez marquée, et il
faut écrire en grand « réel » en haut et « simulé » en bas.

**Changelog (moteur commun, donc les 10 identités en profitent)** :
- caméra en trois-quarts plongeante (yaw −0,32, pitch 0,20, focale plus ouverte
  0,98) : la demande part du coffre au premier plan et file vers le
  destinataire au fond. Les cadrages par phase restent relatifs à cet angle ;
- la surface devient une **dalle épaisse** (tranche visible) qui s'étend loin
  derrière (z −14 → 4) : la perspective du sol se lit enfin. Les cernes du
  chêne sont des cercles complets coupés aux bords de la dalle ;
- **ombres portées** du coffre et du destinataire, sous une lumière en haut à gauche ;
- faces de verre modelées : dégradé vertical, faces arrière à 35 %, contraste
  lumière/ombre plus fort (`depthK` 1,8 pour Chêne) ;
- **pare-feu épais** (une vraie dalle, plus une feuille) ; ticket du motif épaissi ;
- **« réel »** écrit en très grand sur le mur du fond, au-dessus du plan, et
  **« simulé »** écrit en grand devant, dans le monde du reflet. Les deux sont
  en vrai texte 3D (`R.text3`, posé dans l'espace et déformé par la
  perspective), en contour léger pour ne pas couvrir la scène. Les petits
  libellés « réel · NEAR / reflet · bac à sable » du bord droit sont retirés.

**Implémentation** : `PROMPT-OPENCODE-V7.md`, prérempli pour `chene`, à coller
dans opencode.

**Annulé le 2026-11-11** : Gaia a finalement préféré la Menthe néon, et ne veut
pas des mots « réel » / « simulé » en 3D. `PROMPT-OPENCODE-V7.md` est marqué
périmé. La V7.1 reste dans `planches-v7/` comme historique.

---

## V8 — 2026-11-11 · Menthe néon, 10 façons d'être moderne (`planches-v8/`) — EN REVUE

Demande de Gaia : repartir de la **Menthe néon**, écrire « réel » en haut et
« simulé » en bas **à plat** (plus en perspective), et proposer 10 planches qui
s'appuient sur l'existant mais nettement plus modernes.
Point d'entrée : `planches-v8/index.html`.

**Base** : copie du moteur V7.1 (`renderer8.js`, `shell8.js`, `reflet8.js`,
`themes8.js`, `base/` inchangé). Même scène, mêmes données, même contrat avec
`live.js`. La 3D renforcée de la V7.1 est gardée (vue de trois-quarts, dalle
épaisse, ombres, verre modelé, pare-feu épais).

**Nouveautés du moteur** :
- « réel » / « simulé » dessinés à plat sur l'écran, derrière la scène, en 4
  dispositions : à droite de part et d'autre de la ligne du plan, géants au
  centre, à la verticale le long du bord droit, ou fixes en haut et en bas.
  Ils s'estompent pendant le banc ;
- halo lumineux (bloom) : l'image est réduite, seuillée, floutée puis
  ré-additionnée, donc seules les zones lumineuses rayonnent ;
- matières de volume : verre ombré, **tranches** horizontales, **nuage de points** ;
- sols : alvéoles qui s'allument, dalles, lignes de vitesse, courbes de niveau ;
- fonds : aurore (nappes de lumière qui dérivent), projecteur, faisceaux ;
- habillage : barre de commande en verre dépoli, bouton contour, format
  cinéma (bandes noires), police condensée, verdict géant à plat ;
- sous-étapes du bac à sable raccourcies : clone · clé · joué · reçus · diff.

| # | Planche | Accent / fond | Ce qui la distingue |
|---|---|---|---|
| 01 | Menthe nette | `#3DFFB0` / `#010605` | la Menthe néon choisie, nettoyée : grille néon, pluie, halo |
| 02 | Aurore | `#3DFFB0` / `#020807` | nappes de lumière, commande en verre dépoli, mots géants en filigrane |
| 03 | Tranches | `#3DFFB0` / `#010504` | volumes en tranches de lumière, police condensée, mots à la verticale |
| 04 | Nuage de points | `#3DFFB0` / `#010404` | volumes en points, étoiles, typographie très fine |
| 05 | Brutal | `#3DFFB0` / `#020403` | verdict géant à plat, traits épais, aucun halo, dalles |
| 06 | Laque noire | `#3DFFB0` / `#000000` | noir pur, sol miroir, vue basse grand angle, typographie fine |
| 07 | Hexa | `#3DFFB0` / `#010605` | sol en alvéoles vivantes, vue plus haute, prismes hexagonaux |
| 08 | Cinéma | `#3DFFB0` / `#010504` | bandes noires, grand angle, pluie et faisceaux, halo fort |
| 09 | Topo | `#7CFFD0` / `#020B09` | courbes de niveau, menthe plus claire, verre dépoli |
| 10 | Givre | `#D6FFF0` / `#04221B` | la menthe devient le fond, verre givré, accent blanc-menthe |

**Vérifié** : les 10 planches chargent et jouent sans erreur (captures figées
du repos au banc, plus une capture mobile de la 01).

**Piste ouverte** : Gaia autorise three.js « si ça rend beaucoup plus beau ».
Cela apporterait du vrai verre, de vrais reflets au sol, de vraies ombres et
une profondeur de champ. Coût : un fichier three.js embarqué dans `web/`
(aucun CDN, aucun réseau), donc une dépendance à faire valider côté code.
À faire sur UNE planche, une fois celle-ci choisie.

**Retour attendu** : la planche à pousser (mélanges possibles, ex. sol d'Hexa +
typographie de Laque), puis décision three.js.

### V8.1 — 2026-11-12 · épaisseurs + page d'accueil

Retours de Gaia : « l'index ne montre rien » et « certains éléments n'ont même
pas d'épaisseur ».

**Changelog** :
- `index.html` refait sans script : une grande capture par planche
  (`thumbs/NN.jpg`, moment du verdict ALLOW) ; `apercu-v8.jpg` réunit les 10 ;
- tout ce qui était plat prend du volume (moteur commun, donc les 10 planches) :
  - les 5 anneaux du bac à sable deviennent des bagues épaisses (0,4) ;
  - le sceau du certificat : 7 segments extrudés (0,4) ;
  - la fenêtre 24 h autour du coffre : 24 petites tuiles hautes de 0,24 ;
  - R1–R12 : des plots posés sur la dalle ; celui qui mord se dresse (jusqu'à 1,2) ;
  - les flux d'effets : des rubans larges de 0,44 au lieu de lignes ;
  - les barres R12 : de vraies barres ;
  - la dalle : tranche de 0,7 avec ses deux flancs ;
  - pare-feu 0,55 (au lieu de 0,22), ports CLI/HTTP/MCP qui dépassent des deux
    côtés, ticket du motif 0,5, panneaux du banc 0,45, téléphone 0,26 ;
  - un rail porte les blocs de la chaîne ; le cube de l'agent a des faces pleines ;
- correction de robustesse (`shell8.js`) : le pas de temps ne peut plus être
  négatif. Sur une machine lente, la première image pouvait projeter la caméra
  très loin (scène vide ou vue de travers). Les instants figés (`?t=`) posent
  maintenant la caméra directement. **À signaler à opencode** : le même calcul
  existe dans `web/scene/shell.js` (`dt` non borné à 0), à corriger au portage.

**Vérifié** : les 10 planches recapturées sans erreur après ces changements
(`apercu-v8.jpg`), plus Menthe nette aux instants WARN et BLOCK.

### V8.2 — 2026-11-12 · « Laque noire » retenue — PAQUET PRÊT À DÉPOSER

Choix de Gaia : **06 Laque noire** (noir pur `#000000` + menthe `#3DFFB0`,
sol miroir sans motif, vue basse en grand angle, typographie fine, bouton en
contour, « réel » en haut et « simulé » en bas à plat, en contour).

**Livré** : `G-DESIGN/web-laque/` = la page de production complète, prête à
déposer dans `web/` :
- `index.html` (même en-tête et même `<noscript>` que `web/index.html`,
  nouvelle liste de scripts) ;
- `scene/renderer.js`, `scene/shell.js` (moteur V8.1, avec la correction du
  pas de temps) ; `scene/reflet8.js` (la scène) ; `scene/theme.js` (l'identité,
  un seul thème : c'est le seul fichier à modifier pour changer de couleurs) ;
- `scene/data.js`, `timeline.js`, `live.js` : copies de `web/scene/` du
  2026-11-12, présentes seulement pour que le paquet tourne tel quel. Elles ne
  sont pas à recopier.

En production, les liens en haut à droite redeviennent « tableau de bord »
(`classic.html`) et « aide », et la pastille de couleur des planches disparaît.

**Implémentation** : `PROMPT-OPENCODE-V8.md` (dépôt des fichiers + vérification
du mode LIVE). Claude n'a pas modifié `web/`.

**Non fait** : la version three.js (vrais reflets au sol), à décider après ce dépôt.

---

## Commandes design (rappel)

`/planches N` · `/variantes` · `/delta` · `/cible <écran>` · `/compare A B` · `/lock`
