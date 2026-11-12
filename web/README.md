# Interface web de Dry Run

Page statique servie par le garde-barrière : **aucun CDN, aucune dépendance
externe, aucune image, aucun accès réseau**. Tout est en canvas 2D + CSS.

La page d'accueil (`index.html`) est la scène 3D **« Plateau »** (design V9) : vue
de dessus, comme un plateau de jeu. La demande suit une route au sol en
serpentin, à travers sept étapes numérotées : **1 agent IA → 2 pare-feu → 3 bac
à sable** (une boîte de verre en pointillés qui contient les copies du coffre et
du destinataire) **→ 4 règles → 5 verdict → 6 coffre** (le certificat se pose
au-dessus) **→ 7 chaîne NEAR** → destinataire. Chaque élément a sa place ; aucun
volume n'en traverse un autre.

Disposition de l'écran : en haut, **une seule étiquette blanche**. Au repos, elle
dit ce que fait Dry Run, avec une petite ligne indiquant le mode (démo scriptée
ou mode réel). En cours d'usage, elle dit en une phrase ce qui se passe. Sous
elle, les **menaces en boutons** (une demande chacun). À droite, un **terminal en
verre poli** journalise chaque événement ; son en-tête rappelle le mode. Tous les
textes de la scène sont posés à plat sur des plaques, par-dessus la 3D, et placés
sans jamais se chevaucher (`scene/labels.js`).

Identité visuelle : **« Laque noire »**. Deux couleurs seulement, noir pur
`#000000` et menthe `#3DFFB0` ; tout le reste est du blanc en opacité. Les
couleurs tiennent dans `scene/theme.js`, la position des éléments dans
`scene/layout.js`.

Il n'y a **qu'une seule interface** : la scène 3D. Sans JavaScript, le pare-feu
se teste en ligne de commande (`make demo`, `make attack`, `make sim`).

## Deux modes, détectés automatiquement

| Mode | Quand | Ce qui se passe |
|---|---|---|
| **LIVE** | la page est servie par `dryrun-gate` (`/api/status` répond) | le menu **scénario** propose chaque demande **au vrai pare-feu** ; la scène joue les phases avec la vraie réponse (verdict, règles, effets, certificat, exécution) et le terminal affiche les appels HTTP réels |
| **DÉMO** | page ouverte sans serveur (`file://`, ou API injoignable) | le film scripté de `make demo` joue avec les données réelles du dépôt ; le menu **scénario** permet de jouer le film complet ou chaque scénario seul |

```bash
# à la racine du dépôt — le bac à sable NEAR n'est pas nécessaire pour l'interface
cargo run -p dryrun-gate -- serve --port 8787 --sim stub
# puis ouvrir http://localhost:8787
```

Options : `--port <n>` (défaut 8787), `--web-dir <dossier>` (défaut `web`) et
`--sim auto|real|stub` (`stub` joue la démonstration sur un coffre en mémoire,
sans `near-sandbox`).

## Fichiers

| Fichier | Rôle |
|---|---|
| `index.html` | la scène (charge les modules de `scene/`) |
| `scene/data.js` | données réelles du dépôt : coffre, R1–R12, 5 sous-étapes du bac à sable, 7 vérifications du certificat, banc 200 robots (graine 42), scénarios de `make demo` |
| `scene/timeline.js` | chronologie : beats construits depuis les scénarios, `DR.at(t)` = état complet à l'instant *t* |
| `scene/renderer.js` | renderer 3D canvas : caméra, projection, primitives, volumes de verre ombrés, brume de profondeur, tri en profondeur |
| `scene/labels.js` | calque d'étiquettes : textes à plat sur plaques, placés par priorité sans chevauchement, trait de rappel si une étiquette s'éloigne de son élément |
| `scene/shell.js` | montage : canvas, l'unique étiquette (pitch puis phrase de l'étape), boutons des menaces, lecteur, caméra (glisser, molette, double-clic), terminal en verre |
| `scene/term.js` | terminal live à droite : journalise chaque beat de la chronologie (démo et live) ; `DR.term.log/sep/setMode` |
| `scene/scene.js` | la scène, pilotée par un thème et un agencement : `DR.scene9(DR.THEME, DR.LAYOUT)` |
| `scene/layout.js` | l'agencement « Plateau » : position de chaque élément, route au sol, caméra |
| `scene/theme.js` | l'identité « Laque noire » : couleurs, verre, phrase d'accueil. Seul fichier à modifier pour changer d'identité |
| `scene/live.js` | mode LIVE : `/api/status`, `/api/log`, `/api/propose`, `/api/freeze`, et la traduction d'une réponse réelle en scénario jouable |

## Do X → see Y (mode LIVE)

1. `cargo run -p dryrun-gate -- serve --port 8787 --sim stub`, ouvrir
   `http://localhost:8787` → l'étiquette du haut affiche **« Mode réel »**, l'en-tête
   du terminal **`live · stub`** et la ligne `pare-feu connecté · stub` ; sous
   l'étiquette, un bouton par demande réelle, plus **▶ jouer le processus** pour
   le film de démo.
2. Bouton **« swap good-dex »** → la demande entre dans le bac à sable, les 5
   sous-étapes s'allument (le terminal les détaille une à une), les effets
   apparaissent, R1→R12 défilent → **WARN (R5, premier contact)** → le
   propriétaire confirme (`confirm:true`) → **re-simulation** → **ALLOW** → le
   sceau Ed25519 se ferme (7/7) → la transaction part sur la chaîne → le même
   flux se rejoue en vrai, du coffre au destinataire, et **« sand = réel »** s'affiche, avec le
   hash réel dans le terminal.
3. Bouton **« injection 49 Ⓝ »** → **BLOCK (R1)** : une onde blanche part des règles, la
   chaîne reste **intacte**, rien n'est signé ; la boucle MCP est montrée comme
   **côté agent** (le pare-feu ne reformule jamais).
4. Bouton **« piège drainer »** → **BLOCK (R8)** : les fonds sortent, rien ne
   revient — ce sont les effets qui l'arrêtent, pas une liste blanche.
5. Bouton **« piège two-face »** → le contrat est honnête sous 0,1 NEAR et
   voleur au-dessus : seule la **simulation différentielle (R12)** le démasque,
   la liste blanche ne dit rien de ses deux visages.
6. Bouton **« geler le coffre »** → `POST /api/freeze` → le coffre passe
   **FROZEN** et toute demande suivante est refusée (`ERR_FROZEN`).
7. Le coffre affiché (solde, anneau du plafond 24 h, anneau du journal) vient
   de `GET /api/status`, rafraîchi toutes les 3 s.

## Do X → see Y (mode DÉMO, sans serveur)

1. Ouvrir `web/index.html` directement (double-clic) → badge
   **« démo scriptée »**, le terminal annonce le mode démo.
2. Cliquer **« ▶ jouer le processus »** (ou barre espace) → le film complet de
   `make demo` : swap honnête (WARN → approbation → ALLOW), injection (BLOCK R1
   + boucle MCP côté agent), drainer déjà approuvé (BLOCK R8), vol de clé d'agent
   (refus on-chain `ERR_OVER_TX_CAP`), gel d'un geste, puis le **banc de 200
   robots** (graine 42 : 100 honnêtes exécutés, 100 attaques arrêtées,
   précision 1,00 · rappel 1,00 · F1 1,00 · jugement 0 µs médian / 2 µs p95).
3. Boutons sous l'étiquette → jouer chaque scène séparément : swap honnête, injection
   de prompt, piège drainer, vol de clé d'agent, gel d'un geste.
4. `?t=42` fige la scène à 42 s (arrêt sur image pour la vidéo) ; `?play` démarre
   automatiquement ; `?mode=demo` force le mode démo même derrière le serveur.

## Commandes

Glisser = orbite (bornée) · molette = zoom · double-clic = recentrer ·
espace = play/pause · ← / → = ±2 s.

## API consommée en mode LIVE

| Route | Usage |
|---|---|
| `GET /api/status` | solde, plafonds, dépensé 24 h, gel, contrats connus, backend |
| `GET /api/log` | longueur du journal (anneau des 100 entrées) |
| `POST /api/propose` | la demande ; renvoie `verdict`, `rules`, `card_view` (deltas NEAR et jetons, reçus), `executed`, `note`, `cert` |
| `POST /api/freeze` | gel d'urgence |

Les montants circulent en **chaînes décimales yoctoNEAR** et sont manipulés en
`BigInt` côté page : aucun flottant, aucune perte de précision.

## Limites assumées

- En mode LIVE, **R12 (simulation différentielle)** n'est pas jouée : l'API HTTP
  n'expose que `propose` simple. La divergence au centième reste visible en mode
  DÉMO (données de `make sim`).
- En mode LIVE, le **vol de clé d'agent** n'est pas rejoué non plus (il
  nécessiterait d'appeler le coffre en contournant le garde-barrière) : il reste
  dans la démo scriptée.
- Le propriétaire qui confirme un `WARN` est, dans la démo locale, le même
  processus que le garde-barrière (raccourci assumé, documenté dans
  `web/README.md` d'origine et dans le README du dépôt). En production, cette
  approbation se fait depuis l'appareil du propriétaire.
- La clé de gel ne peut **que** geler (invariant I7) : le bouton de gel reste le
  seul geste utilisable quand tout le reste est refusé.
