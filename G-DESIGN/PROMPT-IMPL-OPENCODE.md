# PROMPT-IMPL — implémentation du design V6 dans `web/` (pour un chat opencode)

> À coller tel quel dans un **nouveau chat opencode**. Rédigé le 2026-11-11
> après la V6 de Claude (`G-DESIGN/planches-v6/`). La planche à implémenter est
> celle que Gaia a retenue (`/lock`) ; par défaut, la référence **01-reflet**.

---

## Le prompt

```
Projet : dry-run — NEAR HACKATHON « Build Beyond Boundaries », track `security`.
Repo : HACKATONS/2026-11-15-near/Security/dry-run/ (dépôt public GaiaSG94/dry-run).
Tu es opencode : tu écris le code. Le design (G-DESIGN/, DESIGN.md) est la
propriété de la session Claude : ne modifie JAMAIS G-DESIGN/ ni contracts/ ni
scripts/. Lis ces fichiers, ne les touche pas.

MISSION
Remplacer l'interface actuelle (web/index.html + style.css + app.js, un
dashboard 2D générique) par le design V6 de G-DESIGN/planches-v6/ — une scène
3D en canvas pilotée par une chronologie — branchée sur le VRAI pare-feu
(serveur HTTP de dryrun-gate), avec un mode démo scripté quand le serveur ne
tourne pas. Zéro dépendance, zéro CDN, zéro image (tout canvas/CSS/SVG).

CONTEXTE HACKATHON (régit les priorités)
- Repo public + post de démo X avant 2026-11-14 23:59 UTC.
- Screening IA (Claude Code / Codex) : le README doit donner des étapes
  « do X → see Y » que le juge peut suivre. C'est ta spec d'acceptation.
- Local-only accepté (Q-012) si le repo est reproductible et la vidéo montre
  un moment complet : action utilisateur → résultat visible.
- Track `security` : « spending limits, policy engines, sandboxed wallets ».

ÉTAT DU DÉPÔT (déjà en place, ne pas casser)
- Cargo workspace : crates/dryrun-core, dryrun-sim, dryrun-gate ; contracts/.
- dryrun-gate sert web/ et expose : GET /api/status, GET /api/log,
  POST /api/propose, POST /api/freeze (+ statiques). Le contrat exact des
  corps est dans crates/dryrun-gate/src/http.rs (à relire avant de coder).
- make web-smoke vérifie la page web (11 contrôles) : à mettre à jour.
- make web-ui-smoke lance la page et joue des scénarios.
- web/README.md documente la page : à réécrire.

LE DESIGN À IMPLÉMENTER
Source : G-DESIGN/planches-v6/ (propriété Claude, lecture seule pour toi).
- core.js : les données réelles (vault, règles R1-R12, étapes du bac à sable,
  7 vérifications du certificat, bench_matrix graine 42, scénarios de make
  demo), la chronologie DR.beats (~72 s, un bouton joue tout), DR.at(t) (état
  complet à l'instant t : phase, scénario, passes, effets, règles allumées,
  verdict, journal, dépense 24 h, gel), et un Renderer 3D canvas (caméra
  orbite/zoom, projection, boîtes de verre, texte écran fixe).
- 01-reflet.html : la planche de référence (conf = {accent, bg, camera,
  draw(R,s), overlay, background, init, cameraAt}) — c'est la structure à
  suivre pour toute la page.
- Les 9 autres planches (02-sas, 05-balance…) : même conf, autres scènes.
  Laquelle implémenter ? Celle lockée par Gaia (interroger avant de commencer ;
  défaut si pas de réponse : 01-reflet).

CE QU'IL FAUT CONSTRUIRE
1. web/scene/ : une copie trimée et documentée du moteur (données, beats,
   renderer), découpée en fichiers < 500 lignes, fonctions < 50 lignes,
   commentaires en français. Ne JAMAIS référencer G-DESIGN depuis web/.
2. web/index.html : la nouvelle page d'accueil = la scène de la planche
   retenue, avec ces différences majeures par rapport à la maquette :
   a. DEUX MODES, autodétectés au chargement :
      - LIVE : fetch('/api/status') répond → la page est servie par le
        pare-feu réel. Badge « live · backend=stub|dryrun-sim ».
      - DÉMO : pas de serveur (page ouverte en file://) → la chronologie
        scriptée actuelle joue avec les données de make demo/make sim.
        Badge « démo scriptée ».
   b. EN LIVE, le bouton ne rejoue pas un film : il PROPOSE réellement.
      - Chaque scénario de la maquette devient un bouton de scénario réel
        (swap good-dex · injection 49 NEAR · drainer · freeze) qui construit
        le corps exact de POST /api/propose (montants en chaînes yoctoNEAR
        via BigInt, jamais de flottant) et PLAYE la chronologie de phases
        (send → sandbox → effects → rules → verdict → cert → chain → proof)
        avec les DONNÉES DE LA RÉPONSE : verdict réel, rules[] réelles,
        card_view.near_deltas / ft_deltas réels, receipts, cert réel
        (digest, expiration, nonce), executed, note.
      - WARN réel : la réponse contient verdict=WARN → phase « approve »
        montrée, puis relance de /api/propose avec confirm:true (le gate
        fait alors re-simulation + exécution — voir http.rs:api_propose).
      - BLOCK : phase « blocked » + note du gate ; pour l'injection, la
        boucle MCP est montrée avec l'étiquette « côté agent — hors pare-feu ».
      - R12 différentiel : non exposé par l'HTTP en LIVE → phase masquée en
        LIVE, conservée en mode DÉMO uniquement.
   c. L'état du coffre (solde, plafond 2/10, dépensé 24 h, gel, contrats
      connus, journal 100) vient de GET /api/status (poll 3 s) et pilote les
      objets 3D : niveau de réserve, anneau 24 h, badge gelé.
   d. La frise du journal vient de GET /api/log (entrées réelles : verdict,
      rule_ids, executed) ; le bouton FREEZE fait POST /api/freeze puis
      reflète l'état gelé (les propositions suivantes afficheront ERR_FROZEN).
   e. Bouton de bascule « moteur : démo scriptée | pare-feu local » + champ
      port (défaut 8787). Erreur réseau → repli démo + message net.
3. web/classic.html : l'ANCIENNE page (dashboard 2D) déplacée telle quelle,
   liée discrètement depuis la nouvelle (repli si le canvas ne plaît pas).
4. web/README.md réécrit : étapes « do X → see Y » pour le juge IA, avec le
   mode LIVE (--sim stub) et le mode DÉMO.
5. make web-smoke : mettre à jour les contrôles vers la nouvelle page
   (présence de la scène, des 4 routes API dans le JS, du lien classic),
   et vérifier que web-ui-smoke joue toujours.

CONTRAT API RÉEL (à relire dans crates/dryrun-gate/src/http.rs avant de coder)
- GET  /api/status → { account_id, balance, tx_cap, daily_cap, spent_24h,
  is_frozen, owner, known_contracts: {compte: empreinte}, backend } —
  montants en CHAÎNES yoctoNEAR.
- POST /api/propose body { kind:"transfer"|"function_call", to?, amount
  (yocto), contract?, method?, args?, deposit?, gas?, intent:{summary?,
  beneficiaries?:[compte], min_in?:[[jeton, montant]], max_out?}, confirm? }
  → { verdict:"ALLOW"|"WARN"|"BLOCK", rules:[{id,title,detail,severity}],
  card (texte), card_view:{summary, near_deltas:[{account,delta_yocto,
  is_negative,formatted}], ft_deltas:[{token,account,delta}], keys:[],
  receipts:{total,failed,stranded}, rules:[...], height, gas_burnt,
  rendered_terminal}, executed, note, cert:{digest,signature,public_key,
  snapshot_height,expires_height,nonce}|null }
- WARN + confirm:true → re-simulation puis exécution (confirm=false sinon).
- POST /api/freeze → VaultStatus gelé.
- Test sans bac à sable : cargo run -p dryrun-gate -- serve --port 8787
  --sim stub (coffre en mémoire, la démo joue quand même — parfait pour le
  mode LIVE sans near-sandbox).

RÈGLES DE CODE DU DÉPÔT (impératives, CI bloquante)
- Fichier < 500 lignes, fonction < 50 lignes, commentaires français,
  identifiants anglais, zéro duplication (factoriser dans le moteur scène),
  zéro unwrap côté Rust (tu ne touches pas au Rust).
- Cargo : cargo fmt --all --check et cargo clippy --workspace --all-targets
  -- -D warnings doivent rester verts (tu ne touches pas au Rust, mais la CI
  tourne sur tout le workspace — ne casse rien d'autre).
- CI < 5 min : tes changements web sont statiques + scripts — ok.
- Aucun secret, aucun CDN, aucun fichier binaire.

ACCEPTATION (do X → see Y — c'est ce que le juge IA suivra)
1. `cargo run -p dryrun-gate -- serve --port 8787 --sim stub` puis ouvrir
   http://localhost:8787 → la scène 3D se charge, badge « live · stub ».
2. Cliquer le scénario swap → la vraie réponse du gate : WARN R5, phase
   approbation, puis ALLOW avec le certificat réel (digest/nonce affichés),
   carte des effets avec les deltas réels, « simulé == réel ».
3. Cliquer l'injection 49 NEAR → BLOCK R1, « rien n'est signé », note du gate.
4. Cliquer FREEZE → badge FROZEN ; toute proposition suivante → ERR_FROZEN.
5. Ouvrir web/index.html en file:// sans serveur → badge « démo scriptée »,
   la chronologie joue avec les données de make demo.
6. node build/verify (ou le smoke) → 0 erreur console sur les deux modes.
7. cargo fmt --all --check ; cargo clippy -D warnings ; make web-smoke : verts.

INTERDITS
- Ne pas modifier G-DESIGN/ (Claude), contracts/, crates/, scripts/, .github/.
- Ne pas poster sur X, ne pas rendre le repo public, ne pas soumettre (l'humain
  décide), ne pas committer de secret.
- Ne pas casser l'ancienne page (elle reste accessible en classic.html).

LIVRABLES HACKATHON (rappel pour Gaia, pas pour toi)
Repo public + post X avant 14/11 23:59 UTC · post X = un moment complet
(action → résultat visible) avec @NEARProtocol #NEARHackathon · formulaire
nearhackathon.com/#apply (track `security`) · README « do X → see Y ».
```

---

## Notes pour Gaia (hors prompt)

- **Le design n'est pas encore locké** : V6 est « EN REVUE », 6 planches sur 10
  construites. Le prompt demande d'interroger sur la planche à implémenter
  (défaut : 01-reflet, validée comme référence par Claude).
- **Ordre d'exécution conseillé** : (1) regarder les 10 planches dans
  `planches-v6/index.html`, (2) `/compare A B` sur deux favorites, (3) `/lock`,
  (4) balancer le prompt ci-dessus dans un chat opencode neuf.
- Le mode LIVE est la grosse valeur ajoutée : c'est exactement le « moment
  complet » (action → résultat visible) que le post X et le screening IA
  cherchent — le juge voit le VRAI pare-feu répondre dans la scène 3D.
- La maquette V6 reste la référence visuelle : l'implémentation doit être
  fidèle à `01-reflet.html` (mêmes formes, même rythme), seule la source des
  données change (film scripté → API réelle).
