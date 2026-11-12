# HANDOFF-AMELIORATION — dry-run design V6 → V7 (pour le chat Claude)

> Écrit par opencode le 2026-11-11. Ce fichier contient **tout** ce dont le chat
> Claude a besoin pour améliorer le design : le prompt à coller, l'état exact du
> projet, les règles non négociables accumulées, le contrat technique du moteur,
> et les limites de propriété. Gaia n'a qu'à copier la section 1.

---

## 1. PROMPT À COLLER DANS LE CHAT CLAUDE

```
Projet : dry-run — NEAR HACKATHON « Build Beyond Boundaries », track AI
Security & Defense. Tu es le designer. On améliore la V6.

1. Lis, dans l'ordre :
   - HACKATONS/CLAUDE.md (tes consignes permanentes)
   - HACKATONS/prompts/07-design.md (le pack design : commandes, rounds)
   - HACKATONS/2026-11-15-near/Security/dry-run/G-DESIGN/HANDOFF-AMELIORATION.md
     (CE fichier : état exact, règles, contrat technique, limites)
   - G-DESIGN/RESTART-DESIGN.md §2 (ce que Gaia veut) et §3 (inventaire
     fonctionnel vérifié dans le code)
   - G-DESIGN/DESIGN.md (historique V1→V5 rejetées + V6 en revue)
   - G-DESIGN/planches-v6/core.js et 01-reflet.html (la référence validée)
2. Le code est en LECTURE SEULE : tu ne touches qu'à G-DESIGN/. Jamais web/,
   crates/, contracts/, scripts/, .github/.
3. Ne modifie PAS core.js de façon cassante : l'implémentation opencode
   (PROMPT-IMPL-OPENCODE.md) dépend de son contrat exact (voir §5 du handoff).
   Ajoute, ne renomme pas, ne change pas la forme des données.
4. Avant de produire : pose-moi les deltas (Garder / Changer / Essayer) sur la
   ou les planches que je veux pousser, OU propose-moi toi-même 5 deltas
   numérotés concrets si je te dis juste « améliore ». Pas de /planches 10 de
   plus : on améliore l'existant, on ne repart pas de zéro.
5. Livrables attendus :
   - planches-v7/ (nouveau dossier, jamais d'écrasement de v6) avec la ou les
     planches améliorées, même structure (un HTML autonome + core.js partagé).
   - DESIGN.md : nouvelle section V7 datée + changelog (sans écraser V1→V6).
   - STATUS.md : ta section ## Claude à jour (phase, round, horodatage).
   - La liste exacte des URLs de contrôle pour moi (fichier + ?t=<secondes>
     pour figer chaque moment fort), car tu ne peux pas lancer de navigateur :
     c'est moi qui vérifie visuellement.
6. Fin de session : résumé + prompt relais pour opencode (implémentation) +
   rappel des 6 commandes.

Rappel commandes : /planches /variantes /delta /cible /compare /lock
Deadline : design locké le 14/11 (soumission 14/11 23:59 UTC).
```

---

## 2. État exact du projet (2026-11-11)

- **Produit** : pare-feu à simulation pour wallets d'agents NEAR. Chaque
  transaction est jouée dans un bac à sable local en tant que le coffre, jugée
  sur ses **effets réels** (R1→R12), puis signée (ALLOW + certificat Ed25519)
  ou refusée (BLOCK). Le coffre applique en plus ses plafonds on-chain même si
  le simulateur est contourné. Slogan : « Every transaction happens twice, the
  first time for nothing. »
- **Design** : V1→V5 rejetées (voir DESIGN.md). **V6 en revue** :
  `G-DESIGN/planches-v6/`, 10 métaphores spatiales annoncées, **6 construites** :
  01-reflet (référence validée), 02-sas, 03-coffre, 05-balance, 06-cloche,
  08-crash. **Manquent : 04-prisme, 07-scanner, 09-ecluse, 10-essaim**
  (sous-agents en cours côté session précédente — ne pas les attendre).
- **Moteur commun** : `planches-v6/core.js` (534 lignes) — données réelles,
  chronologie `DR.beats` (~72 s), état à l'instant t `DR.at(t)`, renderer 3D
  canvas, montage `DR.mount(conf)`. Chaque planche = un `conf` avec sa scène.
- **Implémentation** : préparée mais non commencée. Le prompt est dans
  `G-DESIGN/PROMPT-IMPL-OPENCODE.md` (opencode branchera la scène sur le vrai
  `POST /api/propose`, avec mode DÉMO scripté en repli). **Le design doit rester
  implémentable** : voir §6.
- **Concurrence** : opencode et Claude ne doivent JAMAIS écrire sur les mêmes
  fichiers en même temps (collision déjà observée sur demerit). Claude =
  G-DESIGN/ uniquement.

## 3. Ce que Gaia veut (règles non négociables, accumulées sur 6 rounds)

1. **Toute la page est un espace 3D.** Pas une illustration posée sur du texte.
   Si ça ressemble à un dashboard ou à une doc : raté.
2. **2 couleurs maximum** par planche (fond sombre + un accent). ALLOW / WARN /
   BLOCK se distinguent par la **forme** (plein, contour, fissure…), jamais par
   une 3ᵉ couleur. Chaque planche affiche son code couleur dans sa légende.
3. **Très peu de texte.** Le contenu vit dans les **objets** (niveau de réserve,
   anneau 24 h, serrures, fissures, capsules…). Textes à **taille écran fixe
   ≥ 10 px** (jamais mis à l'échelle du monde au point d'être illisible).
4. **Compréhensible d'un seul coup d'œil** : critère n°1, avant le détail.
5. **Un seul bouton** joue tout le processus jusqu'au résultat (~72 s) ;
   glisser = orbite bornée, molette = zoom, double-clic = recentrer, espace =
   play/pause, ←/→ = ±2 s, `?t=12.3` = arrêt sur image.
6. **Tout doit rester visible** (rien ne disparaît) : coffre + plafonds +
   fenêtre 24 h, motif de transaction, bac à sable et ses **5 sous-étapes**,
   effets, R1→R12, verdict, certificat et ses **7 vérifications**, chaîne,
   banc **200 robots** (matrice, F1, latences), journal, surfaces CLI/HTTP/MCP,
   gel, défense on-chain (codes ERR_*).
7. **Le récit** : les deux bouts de la transaction (agent A → destinataire B,
   avec le **type réel** de B : compte, contrat DEX, contrat hostile,
   attaquant), un **exemple concret de motif**, le pipeline, le bac à sable, le
   résultat, puis la suite selon le verdict.
8. **Honnêteté sur la boucle** : le pare-feu ne reformule JAMAIS (BLOCK =
   terminal). Seule re-simulation codée : WARN → `approve_contract` (1 yocto) →
   rejeu. La boucle « l'agent réessaie autrement » est **côté agent via MCP**
   (`wallet_propose`, `wallet_explain`) : à montrer comme telle, jamais comme
   une fonction du pare-feu.
9. **Scénario réel** (celui de `make demo`) : swap good-dex WARN R5 →
   approbation → ALLOW → certificat → exécution → « simulé == réel » ; puis
   injection 49 NEAR → BLOCK R1 (+ boucle MCP) ; drainer déjà approuvé →
   BLOCK R8 (ce sont les effets qui l'arrêtent) ; vol de clé → refus on-chain
   `ERR_OVER_TX_CAP` ; gel ; banc 200/200 F1 1,00 (graine 42).
10. **Une seule idée forte par planche.** Moins d'éléments, plus grands, plus
    lisibles, plus assumés.

## 4. Ce qui a raté avant (à ne pas refaire — détail dans RESTART-DESIGN §6)

- V1/V2 : tableaux de bord documentés, trop de texte, zéro identité.
- V3/V4 : bon récit, forme = colonnes de texte 2D.
- V5 : 13 panneaux de verre flottants trop petits, scène encombrée, illisible,
  10 variantes d'une même mauvaise idée.
- V6 (à améliorer, pas à jeter) : les métaphores sont justes ; les axes
  d'amélioration probables = hiérarchie visuelle (un seul sujet par plan),
  rythme (temps d'arrêt sur les moments forts), lisibilité des chiffres à
  l'échelle, cadrage caméra par phase, et la lisibilité du moment
  « simulé == réel » (la preuve la plus forte du produit).

## 5. Contrat technique de core.js (à ne pas casser — l'implémentation en dépend)

- `DR.mount(conf)` : `conf = { id, name, idea, accent, bg, camera, centerY,
  legend:[[texte, style],…], hello, init(R), background(R,s), draw(R,s),
  overlay(R,s), cameraAt(s,R) }`.
- `DR.at(t)` → état complet : `{ t, T, g, bi, beat, ph, p, pass, scI, sc,
  sub, subP, caption, verdict, rules, rulesScan, effects, lastPass, scView,
  history, spent, frozen, journal, bench, since(ph,pass), reached(ph,pass),
  k(ph,pass) }`.
- Phases (`DR.PH`) : arrive, send, sandbox (5 sous-étapes), effects, rules,
  verdict, approve, cert (7 vérifications), chain, proof, blocked, mcp, bypass,
  onchain, freeze, bench, end.
- `DR.scenarios` : 5 scénarios réels (swap 2 passes WARN→ALLOW, inject, drainer,
  theft/bypass, freeze) avec `passes[].verdict/rules/effects` (BigInt yocto).
- Renderer `R` : `R.poly/path/line/circle/box/dot/text/raw/flush`, `R.p(x,y,z)`
  (projection), `R.A(a)`/`R.Wh(a)` (couleurs), `R.cam`, `R.user`, `R.narrow`.
- Texte : `R.text` avec `px` (taille écran fixe) **ou** `size` (taille monde,
  bornée min/max) — toujours garder un `min ≥ 10`.
- Interactions déjà câblées par `mount` : play/pause/rejouer, barre de
  progression par scénario, clavier, `?t=`, `?play`, orbite/zoom/dbl-clic.

## 6. Contraintes d'implémentabilité (le design sera codé par opencode)

- La scène sera branchée sur le vrai serveur : `GET /api/status`,
  `GET /api/log`, `POST /api/propose`, `POST /api/freeze` (contrat exact dans
  `crates/dryrun-gate/src/http.rs`). Conserver la chronologie par phases :
  c'est elle qui sera pilotée par la réponse réelle.
- **R12 (différentiel)** : non exposé par l'API HTTP en mode LIVE → le garder
  dans le récit DÉMO, mais prévoir qu'il puisse être masqué en LIVE.
- **WARN** : en LIVE, l'approbation = re-POST avec `confirm:true` (le gate
  re-simule puis exécute). La phase « approve » doit rester une phase à part.
- Zéro CDN, zéro dépendance, zéro image : tout en canvas/CSS/SVG inline.
- Polices : uniquement des piles système (déjà dans `DR.SANS` / `DR.MONO`).
- Page servie par `dryrun-gate serve --port 8787` : pas de module ES, pas de
  build step (fichiers statiques simples, `<script src>` classiques).
- Fichiers < 500 lignes (règle du dépôt) : si une planche dépasse, la découper.

## 7. Processus de travail (rappel du pack design)

- Rounds : versions datées en **ajout** dans DESIGN.md, jamais écrasées ;
  changelog par round ; feedback Garder / Changer (deltas numérotés) / Essayer.
- Au-delà de 4-5 rounds sans convergence : proposer un re-brief, pas un round
  de plus. (On en est à V6 — la V7 doit converger, pas explorer.)
- Gaia vérifie visuellement (Claude n'a pas de navigateur) : fournir les URLs
  `?t=` des moments forts à contrôler.
- Fin de session : résumé + prompt relais + rappel des 6 commandes.

## 8. Fichiers utiles (chemins complets)

- Brief qui fait foi : `…/Security/dry-run/G-DESIGN/RESTART-DESIGN.md`
- Historique design : `…/G-DESIGN/DESIGN.md` (V1→V5 rejetées, V6 en revue)
- Moteur + planches : `…/G-DESIGN/planches-v6/` (core.js, index.html,
  01-reflet, 02-sas, 03-coffre, 05-balance, 06-cloche, 08-crash)
- Prompt d'implémentation opencode : `…/G-DESIGN/PROMPT-IMPL-OPENCODE.md`
- Brief UI initial : `…/G-DESIGN/BRIEF-UI.md`
- Code (lecture seule) : `…/Security/dry-run/` (README.md, docs/SPEC.md,
  docs/SPEC_V1_MAX.md, crates/dryrun-gate/src/http.rs, web/)
- Consignes permanentes Claude : `HACKATONS/CLAUDE.md`,
  `HACKATONS/prompts/07-design.md`
