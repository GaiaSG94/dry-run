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

### 2026-11-12 · 22 h 40 — vérification V9 + demande testnet

- `main` vérifié à l'écran (V9 « Plateau » + HUD de la PR #2) : grande étiquette, ligne « démo / mode réel » avant tout lancement, attaques en boutons, contours 3D épais. Ordinateur 1440 px : aucun chevauchement, la retouche `padT +70` annulée est inutile.
- Mode réel testé contre `dryrun-gate serve` (port 8787, stub) : 49 NEAR vers mallory → BLOCK R1 ; 1 NEAR vers owner → ALLOW (le coffre en mémoire passe à 49 NEAR).
- Mobile 375 px : le mode réel se connecte, mais la scène est petite, les étiquettes se chevauchent et le terminal est absent. Reste design.
- Testnet demandé par Gaia : `dryrun-gate` n'a pas de mode testnet. Le script ne déploie que le contrat. Demande et options consignées dans `G-DESIGN/HANDOFF.md` (recommandation : option A).
- Ajouté : `.claude/launch.json` (aperçu local de `web/`, non commité).

### 2026-11-12 · 23 h — coffre déployé sur testnet (exception : fait par Claude)

- `dryrun-vault-1112.testnet` : compte créé (faucet), `vault` déployé et initialisé (2 NEAR / tx, 10 NEAR / 24 h).
- Vérifié sur la chaîne : 5 NEAR → `ERR_OVER_TX_CAP` ; 0,1 NEAR → exécuté.
- README section « 6 bis » + tableau des limites, branche `docs/testnet-vault`, non commitée.
- Reste : relier `dryrun-gate` au coffre testnet (code).

### 2026-11-13 · nuit — vidéo démo V1 (voix) + page en anglais

- La vidéo brute d'opencode (`video-demo-raw-2026-11-12.mp4`) est **inutilisable** : elle filme l'app Claude, avec des conversations privées. À ne pas publier.
- Page en anglais : `web/scene/i18n.js`, actif seulement avec `?lang=en` (le français reste par défaut). Il traduit le DOM, les étiquettes 3D (canvas) et la boîte de gel. Une ligne ajoutée dans `web/index.html`. Non commité.
- Vidéo V1 : `G-DESIGN/video/dryrun-demo.mp4`, 104 s, 1440×920, voix ElevenLabs (voix clonée), sous-titres dans un bandeau. Script et chaîne : `G-DESIGN/video/SCRIPT.md`.
- Contrôle : −16,9 LUFS, plus long silence 2,3 s, sous la limite X de 2 min 20.
- Retour de Gaia sur la V1 : « pas assez moderne ». Attendus relevés sur nearhackathon.com + HACKATHON.md (problème d'abord, un seul moment complet action → résultat, rôle concret de NEAR, prochaine étape + question ; pas d'intro longue, pas de schéma d'architecture). Pistes V2 proposées, en attente du choix.
- Gaia a choisi la piste A (typo cinétique), puis le scénario « The message ». V2 construite avec HyperFrames : `G-DESIGN/video/v2/film/`, rendu `G-DESIGN/video/v2/dryrun-demo-v2.mp4` (45,5 s). `npx hyperframes check` : 0 erreur, 0 avertissement. `web/scene/i18n.js` complété (« R1 · cap »).
- V3 (retours de Gaia : ouvrir sur le schéma 3D, plus de zoom, meilleure résolution) : `G-DESIGN/video/v3/film/`, page filmée en 3840×2160 au ralenti 0,75, plan fixe, rendu 4K + version 1080p pour X.
- V4 (retours de Gaia : 2:20 pile, tous les cas, problème limpide, NEAR technique sous plusieurs angles avec la 3D visible, stack complète, étiquettes plus grosses) : `G-DESIGN/video/v4/`, `web/scene/labels.js` (`?labels=`) et `i18n.js` (`?stack=1`) modifiés, site par défaut inchangé.
- 2026-11-13 : Gaia valide la V4 (2:20) et choisit le recadrage B « parcours utilisateur » (une personne, l'agent branché par MCP, fin heureuse). Demande à opencode dans `HANDOFF.md` : brancher `dryrun-gate` sur le coffre testnet (~6 h, avant 20h Paris), sinon vidéo B sans signature sur la chaîne. MCP vérifié avec un vrai client (BLOCK R1 / ALLOW) ; 3 corrections demandées (« Exécuté sur chaîne : OUI » trompeur en local, réponses en français, réponse à une notification). Prochaine étape Claude : découpage de la vidéo B, puis tournage dès le retour d'opencode.
- 2026-11-13 : découpage de la vidéo V5 « parcours utilisateur » validé par Gaia (`G-DESIGN/video/v5/STORYBOARD.md`). Ajout, à sa demande, du bouton « payer une facture · 1 Ⓝ » (EN : « pay invoice · 1 Ⓝ ») dans `web/scene/live.js` + traductions dans `i18n.js` : transfert de 1 NEAR vers supplier.test.near, ALLOW vérifié contre le vrai pare-feu (stub), site FR et EN vérifiés. Non commité. Tournage V5 dès le retour d’opencode (testnet + MCP), au plus tard ce soir 20h.
- 2026-11-13 : PR GaiaSG94/dry-run#2 (README testnet) et #3 (page EN, ?labels, ?stack, bouton facture, sources vidéo) fusionnées dans `main` (082aa18). Rendus vidéo hors git (`G-DESIGN/video/.gitignore`). Audit de l’historique : clone complet (22 commits, 1er le 10/11), points d’attention notés pour Gaia (gros premiers commits, README à aligner sur le dépôt, une seule soumission, tag à déplacer).
- Défauts V1 (sans objet si la V2 est retenue) : virgule française dans « 1,96 TEST » et « F1 1,00 », « certificat 1/7 » non traduit dans le terminal, ligne « owner: approval » en double (saut dans le film), parenthèse qui passe à la ligne sur la carte testnet, plaque « FROZEN » visible pendant le banc.

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
