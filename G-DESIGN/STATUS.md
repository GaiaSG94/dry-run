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
