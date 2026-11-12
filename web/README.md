# Interface web de Dry Run

Page statique servie par le garde-barrière : **aucun CDN, aucune dépendance
externe, aucune image, aucun accès réseau**. Tout est en canvas 2D + CSS.

La page d'accueil (`index.html`) est la scène 3D **« Le Reflet »** : une surface
de verre coupe l'espace — au-dessus, NEAR (le réel) ; en dessous, son reflet (le
bac à sable). Chaque demande se joue d'abord dans le reflet ; si elle passe, le
même geste se rejoue au-dessus, en miroir : **simulé == réel**.

Identité visuelle : **« Laque noire »** (design V8.2). Deux couleurs seulement,
noir pur `#000000` et menthe `#3DFFB0` ; tout le reste est du blanc en opacité.
Sol miroir sans motif, vue basse en grand angle, volumes de verre épais.
« réel » est écrit en haut et « simulé » en bas, à plat sur l'écran.
Toute l'identité tient dans `scene/theme.js`.

L'ancienne interface (tableau de bord 2D) reste disponible sur `classic.html`.

## Deux modes, détectés automatiquement

| Mode | Quand | Ce qui se passe |
|---|---|---|
| **LIVE** | la page est servie par `dryrun-gate` (`/api/status` répond) | chaque bouton **propose la demande au vrai pare-feu** ; la scène joue les phases avec la vraie réponse (verdict, règles, effets, certificat, exécution) |
| **DÉMO** | page ouverte sans serveur (`file://`, ou API injoignable) | la chronologie scriptée de `make demo` joue avec les données réelles du dépôt |

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
| `scene/shell.js` | montage : canvas, HUD, lecteur (play/pause/reprise), caméra (glisser, molette, double-clic), halo lumineux |
| `scene/reflet8.js` | la scène « Le Reflet », pilotée par un thème : `DR.reflet8(DR.THEME)` |
| `scene/theme.js` | l'identité « Laque noire » : couleurs, sol, lumière, typographie, caméra. Seul fichier à modifier pour changer d'identité |
| `scene/live.js` | mode LIVE : `/api/status`, `/api/log`, `/api/propose`, `/api/freeze`, et la traduction d'une réponse réelle en scénario jouable |
| `classic.html` + `style.css` + `app.js` | l'ancien tableau de bord 2D (repli) |

## Do X → see Y (mode LIVE)

1. `cargo run -p dryrun-gate -- serve --port 8787 --sim stub`, ouvrir
   `http://localhost:8787` → badge **`live · stub`** en haut à gauche, quatre
   boutons de scénario apparaissent, le bouton principal devient
   « démo scriptée ».
2. Cliquer **« swap good-dex »** → la capsule descend dans le reflet, les 5
   sous-étapes s'allument, les effets apparaissent, R1→R12 défilent →
   **WARN (R5, premier contact)** → le propriétaire confirme (`confirm:true`) →
   **re-simulation** → **ALLOW** → le sceau Ed25519 se ferme (7/7) → la
   transaction part sur la chaîne → le même flux se rejoue au-dessus du verre et
   **« simulé == réel »** s'affiche, avec le hash réel dans la légende.
3. Cliquer **« injection 49 Ⓝ »** → **BLOCK (R1)** : le verre se fissure, la
   chaîne reste **intacte**, rien n'est signé ; la boucle MCP est montrée comme
   **côté agent** (le pare-feu ne reformule jamais).
4. Cliquer **« piège drainer »** → **BLOCK (R8)** : les fonds sortent, rien ne
   revient — ce sont les effets qui l'arrêtent, pas une liste blanche.
5. Cliquer **« geler le coffre »** → `POST /api/freeze` → le coffre passe
   **FROZEN** et toute demande suivante est refusée (`ERR_FROZEN`).
6. Le coffre affiché (solde, plafond 2 / tx, plafond 10 / 24 h, dépensé, anneau
   des 24 heures, anneau du journal, contrats connus) vient de `GET /api/status`,
   rafraîchi toutes les 3 s.

## Do X → see Y (mode DÉMO, sans serveur)

1. Ouvrir `web/index.html` directement (double-clic) → badge
   **« démo scriptée »**, aucun bouton de scénario.
2. Cliquer **« jouer le processus »** (ou barre espace) → le film complet de
   `make demo` : swap honnête (WARN → approbation → ALLOW), injection (BLOCK R1
   + boucle MCP côté agent), drainer déjà approuvé (BLOCK R8), vol de clé d'agent
   (refus on-chain `ERR_OVER_TX_CAP`), gel d'un geste, puis le **banc de 200
   robots** (graine 42 : 100 honnêtes exécutés, 100 attaques arrêtées,
   précision 1,00 · rappel 1,00 · F1 1,00 · jugement 0 µs médian / 2 µs p95).
3. `?t=42` fige la scène à 42 s (arrêt sur image pour la vidéo) ; `?play` démarre
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
