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
