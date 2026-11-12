# STATUS — dry-run (design)

## Claude

- Phase : « Laque noire » (V8.2) **déposée dans `web/`** par Claude, à la demande de Gaia (2026-11-12).
- Sauvegarde de l'ancien `web/` : `G-DESIGN/web-avant-laque-2026-11-12/`.
- Fichiers de `web/` modifiés : `index.html`, `README.md`, `scene/renderer.js`, `scene/shell.js`, `scene/live.js` ; ajoutés : `scene/reflet8.js`, `scene/theme.js` ; supprimé : `scene/reflet.js` (plus chargé, présent dans la sauvegarde).
- Vérifié contre le vrai serveur (`dryrun-gate serve --port 8791 --sim stub`) : badge live, 4 scénarios, gel, `classic.html`, démo forcée.
- Corrigé dans `scene/live.js` : minimum TEST exprimé en 24 décimales au lieu de 18 (le swap honnête finissait toujours en BLOCK R8) ; affichage des montants TEST.
- Corrigé dans `scene/shell.js` : halo qui plantait sur une page chargée masquée ; ligne du halo neutralisée par un commentaire ; pas de temps négatif.
- **À signaler à opencode** : avec `--sim stub`, le simulateur ne renvoie aucun delta de jeton, donc le swap good-dex est BLOCK R8 en live, contrairement à ce qu'annonce `web/README.md` (WARN → ALLOW). Le chemin ALLOW n'a été testé qu'avec une réponse simulée côté page.
- Non fait : `make test` / `make lint` (make absent de cette machine ; aucun fichier Rust modifié), commit Git, `DESIGN.md` (section « dépôt dans web/ » à ajouter), halo des autres planches V8 à recapturer, rendu mobile, version three.js.
- Le paquet `G-DESIGN/web-laque/` n'a pas reçu la correction de `live.js` (il ne devait pas être copié).

### 2026-11-12 · soir — HUD en haut (fusionné)

- PR #1 (cadrage : plus rien sous le terminal) et PR #2 (HUD) **fusionnées dans `main`** (squash, CI verte).
- PR #2 : étiquette blanche agrandie et en haut, une phrase qui dit ce que fait Dry Run ; bandeau « démo scriptée / vrai pare-feu » visible avant tout lancement ; attaques en boutons sous l'étiquette (fin du menu déroulant) ; contours 3D plus épais, noms plus grands.
- Reste design : rendu mobile (scène 3D minuscule), section « dépôt dans web/ » de `DESIGN.md`, recapture des planches V8.
- Reste avant soumission (14/11 23:59 UTC), hors design : bug MCP (49 NEAR avec champ erroné lu comme 1 NEAR), déclaration IA du README (Claude Code / opencode absents), tag `hackathon-submission`, règle « 2 projets max / README cite l'autre », vidéo + post X, formulaire (compte mainnet), déploiement testnet optionnel.

### 2026-11-12 · nuit — V9 en revue (`planches-v9/`)

- Trois planches variantes : A · Ligne, B · Miroir rangé, C · Plateau (`planches-v9/index.html`), aperçu `apercu-v9.jpg`.
- Communs : une seule étiquette (pitch, puis la phrase de l'étape), menaces en boutons, étiquette technologies supprimée, terminal en verre, 7 étapes numérotées, textes à plat sans chevauchement.
- Gaia retient **C + verre poli** : déposé dans `web/` (branche `feat/web-plateau-v9`), `reflet8.js` retiré.
- Vérifié en live (`--sim stub`, port 8791) : connexion, ligne « Mode réel » dans l'étiquette, « injection 49 Ⓝ » → BLOCK R1 rejoué par la scène.
- Non fait : rendu mobile ; le swap good-dex reste BLOCK R8 avec `--sim stub` (limite du simulateur allégé).

## opencode

- 2026-11-12 · branche `ui-v8-terminal` (nouvelle, poussée sur origin).
- Commit 1 : dépôt du travail « Laque noire » V8 existant (scène, classic.html, docs G-DESIGN/*.md).
- Commit 2 (demandes Gaia) :
  - terminal live à droite (`scene/term.js`) : chaque beat de la chronologie + appels réseau LIVE y sont journalisés ;
  - éléments 3D agrandis (~1,3×), plateforme rectangulaire resserrée (z −14..4 → −6,5..4,3) ;
  - textes réduits pour zéro chevauchement : suppression de « au-dessus le réel… », légende exécuté/WARN/BLOCK, note bas-droite « pare-feu à simulation · make sim graine 42 », libellés coffre « 2/tx · 10/24h », « 0 FullAccess », points execute/freeze, « jumeau », « clé jetable », sous-titres acteurs, ligne précision/rappel + pièges du banc ;
  - « simulé » → « sand » (mot de fond, preuve `sand = réel`, sous-étapes terminal) ; mots RÉEL/SAND déplacés à gauche (la droite = terminal) ;
  - boutons scénarios → menu déroulant « scénario » (démo : film complet + 5 scènes ; live : 4 demandes réelles + démo) ;
  - étiquette blanche à écriture verte (événement en cours, ex-#cap) + étiquette technologies similaire (logos NEAR/Rust simple-icons + Ed25519 · MCP) ;
  - lien « tableau de bord » retiré du HUD (classic.html reste en repli noscript) ; nav réduite à « aide ».
- Vérifications : `node --check` sur 9 fichiers JS OK ; test runtime jsdom (faux gate HTTP + canvas stubbé) : 22/22 PASS, 0 erreur (mode live, WARN→confirm→ALLOW, menu, terminal, démo restaurée, banc, playScenario).
- Répondu à Gaia : la scène + la démo scriptée marchent 100 % côté client hors serveur ; le mode LIVE exige que la page soit servie par `dryrun-gate` (sinon repli démo automatique).

## opencode (suite)

- Suppression du repli classique demandée par Gaia : `web/classic.html`, `web/app.js` et `web/style.css` sont supprimés ; il n'y a plus qu'une interface (la scène 3D). Le message `<noscript>` renvoie désormais vers la ligne de commande (`make demo` / `make attack` / `make sim`).
- Conséquence Rust : `crates/dryrun-gate/tests/http_api.rs` (`page_index_est_servie`) attendait `app.js` dans la page servie ; l'assertion vérifie désormais `scene/shell.js` (la page servie est la scène).
- Le scénario **two-face** qui venait d'être ajouté au tableau de bord 2D est porté dans le menu « scénario » de la scène (`web/scene/live.js`) : il ne perd pas son accès 1 clic.

## opencode (2026-11-12 · soir) — clone GaiaSG94 + fix MCP

- Dépôt publié en miroir sur <https://github.com/GaiaSG94/dry-run> : repo **neuf** (pas un fork, aucun lien de dépendance affiché), historique complet poussé (`main` + `ui-v8-terminal`).
- **Fix bug MCP** (`crates/dryrun-gate/src/mcp.rs`) : `wallet_propose` rejette désormais toute demande mal formée AVANT simulation — champ inconnu (ex: `amount` au lieu de `amount_near`), `intent` manquant, montant absent ou non numérique, `kind` inconnu, `to`/`contract`/`method` manquants. Avant : 49 NEAR envoyés avec un champ mal nommé étaient lus comme 1 NEAR et acceptés.
- Tests : 6 tests d'intégration dans `crates/dryrun-gate/tests/mcp.rs` (dont la régression 49 NEAR) ; suite `cargo test -p dryrun-gate` verte (tous les suites passent).
- README : §10 déclare Claude Code et opencode dans l'assistance IA ; ligne de période (premier commit 10/11, soumission 14/11 23:59 UTC) ajoutée en tête.
- `scripts/record-demo.sh` : plus d'IP ni de nom de machine en dur — paramétré par `DRYRUN_GATE_URL` (défaut 127.0.0.1:8787) et `DRYRUN_DEMO_HOST` (vide = étape d'ouverture d'écran sautée). Syntaxe bash vérifiée (git-bash).
- Tag `hackathon-submission` posé sur `main` et poussé sur les deux remotes.
- **Vidéo brute enregistrée** : `G-DESIGN/video-demo-raw-2026-11-12.mp4` (42 s, 1280x736, 30 i/s) — serveur `--sim stub` port 8787 + fenêtre Chrome `--app` + capture ffmpeg gdigrab, `scripts/record-demo.sh` déroulé en entier (2 ALLOW, 2 BLOCK, gel final). Non commitée (brute, à monter selon `prompts/05-video.md`).
- Note opérationnelle : le serveur `dryrun-gate serve --port 8791 --sim stub` tournant a dû être arrêté pour relinker le binaire pendant les tests ; relancé ensuite avec le binaire reconstruit, puis arrêté après la capture.
