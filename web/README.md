# Interface web de Dry Run

Page statique servie par le garde-barrière : **aucun CDN, aucune dépendance
externe, aucune image, aucun accès réseau**. Tout est en canvas 2D + CSS.

La page d'accueil (`index.html`) est la scène 3D **« Le Reflet »** : une surface
de verre coupe l'espace — au-dessus, NEAR (le réel) ; en dessous, son reflet (le
bac à sable). Chaque demande se joue d'abord dans le reflet ; si elle passe, le
même geste se rejoue au-dessus, en miroir : **sand == réel**.

Disposition de l'écran : à droite, un **terminal live** dit brièvement chaque
événement (étapes du bac à sable, effets, règles, verdicts, appels réseau en
mode LIVE) ; en bas, une **étiquette blanche à écriture verte** raconte
l'événement en cours, à côté de l'**étiquette des technologies** (NEAR, Rust,
Ed25519, MCP) ; un unique bouton **« scénario »** ouvre un **menu déroulant**
pour choisir quoi jouer.

Identité visuelle : **« Laque noire »** (design V8.3). Deux couleurs seulement,
noir pur `#000000` et menthe `#3DFFB0` ; tout le reste est du blanc en opacité.
Sol miroir sans motif, vue basse en grand angle, volumes de verre épais.
« réel » est écrit en haut à gauche et « sand » en bas à gauche, à plat sur
l'écran. Toute l'identité tient dans `scene/theme.js`.

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
| `scene/shell.js` | montage : canvas, HUD (étiquettes blanches, menu déroulant « scénario », lecteur), caméra (glisser, molette, double-clic), halo lumineux |
| `scene/term.js` | terminal live à droite : journalise chaque beat de la chronologie (démo et live) ; `DR.term.log/sep/setMode` |
| `scene/reflet8.js` | la scène « Le Reflet », pilotée par un thème : `DR.reflet8(DR.THEME)` |
| `scene/theme.js` | l'identité « Laque noire » : couleurs, sol, lumière, typographie, caméra, étiquette technologies. Seul fichier à modifier pour changer d'identité |
| `scene/live.js` | mode LIVE : `/api/status`, `/api/log`, `/api/propose`, `/api/freeze`, et la traduction d'une réponse réelle en scénario jouable |

## Do X → see Y (mode LIVE)

1. `cargo run -p dryrun-gate -- serve --port 8787 --sim stub`, ouvrir
   `http://localhost:8787` → badge **`live · stub`** en haut à gauche, le
   terminal affiche `pare-feu connecté · stub`, le menu **scénario** propose
   les cinq demandes réelles + la démo scriptée, le bouton principal devient
   « démo scriptée ».
2. Menu → **« swap good-dex »** → la capsule descend dans le reflet, les 5
   sous-étapes s'allument (le terminal les détaille une à une), les effets
   apparaissent, R1→R12 défilent → **WARN (R5, premier contact)** → le
   propriétaire confirme (`confirm:true`) → **re-simulation** → **ALLOW** → le
   sceau Ed25519 se ferme (7/7) → la transaction part sur la chaîne → le même
   flux se rejoue au-dessus du verre et **« sand = réel »** s'affiche, avec le
   hash réel dans le terminal.
3. Menu → **« injection 49 Ⓝ »** → **BLOCK (R1)** : le verre se fissure, la
   chaîne reste **intacte**, rien n'est signé ; la boucle MCP est montrée comme
   **côté agent** (le pare-feu ne reformule jamais).
4. Menu → **« piège drainer »** → **BLOCK (R8)** : les fonds sortent, rien ne
   revient — ce sont les effets qui l'arrêtent, pas une liste blanche.
5. Menu → **« piège two-face »** → le contrat est honnête sous 0,1 NEAR et
   voleur au-dessus : seule la **simulation différentielle (R12)** le démasque,
   la liste blanche ne dit rien de ses deux visages.
6. Menu → **« geler le coffre »** → `POST /api/freeze` → le coffre passe
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
3. Menu **scénario** → jouer chaque scène séparément : swap honnête, injection
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
