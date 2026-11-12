# RESTART-DESIGN — dry-run (NEAR, AI Security & Defense)

**À lire par Claude avant toute production. Le design repart de ZÉRO.**

Ce fichier remplace les rounds précédents. Tout ce qui existe déjà dans
`G-DESIGN/` (dossiers `planches/`, `planches-v3/`, `planches-3d/`, et les
versions V1→V5 de `DESIGN.md`) est **à considérer comme rejeté**. Ne pas
reprendre, ne pas « améliorer », ne pas recycler : repartir d'une page blanche.
Ces dossiers peuvent être regardés **uniquement** pour savoir ce qu'il ne faut
pas refaire (voir §6).

Deadline design : **14/11** (soumission 15/11 00:59 Paris). Juges : implémentation,
design, impact, originalité. Repo public, post X + vidéo démo requis.

---

## 1. Prompt à coller dans le nouveau chat Claude

```
Projet : dry-run — hackathon NEAR, track AI Security & Defense.

Le design repart de ZÉRO. Les rounds précédents sont rejetés.

1. Lis HACKATONS/CLAUDE.md et HACKATONS/prompts/07-design.md.
2. Lis HACKATONS/2026-11-15-near/Security/dry-run/G-DESIGN/RESTART-DESIGN.md
   en entier : c'est le brief qui fait foi (il contient le résumé de ce que je
   veux, l'inventaire fonctionnel du projet, et la liste de ce qui a raté).
3. Lis aussi G-DESIGN/BRIEF-UI.md (brief initial) et le README.md du repo.
4. Explore le code en LECTURE SEULE (ne rien modifier hors G-DESIGN/) :
   HACKATONS/2026-11-15-near/Security/dry-run/
5. Ne regarde PAS les anciens dossiers planches/, planches-v3/, planches-3d/
   sauf pour comprendre ce qu'il ne faut pas refaire.
6. Propose d'abord 3 pistes courtes (une phrase + une image mentale chacune),
   attends mon choix, puis produis. Pas de /planches 6 : je veux peu de pistes,
   très fortes.

Règles non négociables : toute la page est un espace 3D (pas une illustration
posée sur du texte), 2 couleurs maximum, fond sombre, verre très transparent,
très peu de texte, toutes les fonctionnalités visibles, un seul bouton qui joue
tout le processus jusqu'au résultat, compréhensible d'un seul coup d'œil.

Rappel commandes : /planches /variantes /delta /cible /compare /lock
```

---

## 2. Ce que je veux (résumé fidèle, dans mes mots à moi)

### 2.1 La forme

- **Toute la page est un espace 3D.** Pas « une partie visuelle + le reste en
  texte ». Le texte n'est qu'un habillage léger : des légendes, des libellés,
  quelques nombres. Si un écran ressemble à un tableau de bord ou à une page
  de documentation, c'est raté.
- **Style très transparent, en 3D** : verre, transparence, profondeur, matière.
  On doit avoir l'impression de regarder à travers des volumes, pas de lire des
  cartes opaques.
- **Extrêmement moderne, avant-gardiste.** Fond **sombre** de préférence.
- **2 couleurs maximum** par piste (un fond sombre + un accent). Tout le reste
  est du blanc en opacité. Pas de palette à 5 couleurs, pas de dégradés arc-en-ciel.
- **Très peu de texte.** Garder seulement l'essentiel en termes de mots — mais
  **tout** en termes de fonctionnalités : rien ne doit disparaître, tout doit
  être rendu visible autrement (forme, position, mouvement, couleur, taille).
- **Chaque étape, chaque élément doit être très compréhensible.** Si je dois
  lire une phrase pour comprendre un bloc, le bloc est mal dessiné.
- **Je dois comprendre de quoi il s'agit en regardant le site d'un seul coup
  d'œil.** C'est le critère n°1. Avant le détail, avant l'exhaustivité.

### 2.2 Le récit à mettre en scène

La page doit raconter **une transaction entre deux parties**, dans cet ordre :

1. **Les deux agents de la transaction** doivent être représentés physiquement
   dans l'espace : l'émetteur (l'agent IA qui veut payer, adossé à son coffre)
   et le destinataire (l'autre bout).
   - Réponse à ma question « est-ce que les deux sont des agents IA ? » : dans le
     code, **seul l'émetteur est un agent IA**. Le destinataire est un compte ou
     un contrat NEAR, qui *peut* être le wallet d'un autre agent mais dont le
     pare-feu se moque. L'interface doit dire la vérité là-dessus (type du
     destinataire visible : compte simple / contrat DEX / contrat hostile /
     attaquant), pas inventer une symétrie.
2. **Un exemple concret de motif de transaction** doit être visible : c'est ce
   qui part de l'agent A (destinataire, méthode, arguments, dépôt, gas) plus
   l'intention déclarée (bénéficiaires, minimum attendu).
3. **Un bouton déclenche tout le processus**, et on le voit se dérouler
   jusqu'au résultat. Pas une animation décorative : le vrai enchaînement.
4. **On voit le pipeline entre les deux agents** : le trajet de la demande.
5. **On voit l'étape « bac à sable »** comme un espace à part (hors chaîne), et
   **on voit visuellement ses sous-étapes** : clone de l'état, clé jetable,
   exécution en tant que le coffre, attente des reçus, diff avant/après.
6. **On voit le résultat du bac à sable** : les effets observés (qui a gagné,
   qui a perdu, clés, reçus) et les règles qui ont mordu.
7. **Selon le résultat, on voit la suite** :
   - `ALLOW` → certificat signé → **la transaction réelle se fait**, visuellement,
     jusqu'au destinataire, avec la preuve « simulé == réel ».
   - `WARN` → le propriétaire approuve → **re-simulation**.
   - `BLOCK` → rien n'est signé, la chaîne reste intacte.
8. **La question de la boucle** (je l'ai posée explicitement) : *si le résultat
   du bac à sable ne convient pas, est-ce que l'agent trouve une autre manière
   et reteste jusqu'à ce que ce soit bon ?*
   **Réponse factuelle, vérifiée dans le code : NON, pas dans le pare-feu.**
   - `BLOCK` est terminal : le pare-feu rend la carte et s'arrête. Il ne
     reformule jamais rien.
   - La seule re-simulation codée est `WARN` → `approve_contract` (1 yocto) →
     `propose` rejoué (`crates/dryrun-gate/src/demo.rs:79` et `:150`).
   - La boucle « l'agent réessaie autrement » n'existe que **côté agent**, via
     le serveur MCP (`wallet_propose`, `wallet_status`, `wallet_explain`,
     `wallet_freeze`) : l'agent reçoit verdict + règles + carte et peut rappeler.
   - **Consigne design** : on peut la montrer, mais elle doit être visuellement
     et textuellement attribuée à l'agent (boucle externe, optionnelle), jamais
     présentée comme une fonction du pare-feu. Ne pas mentir aux juges.
9. **Les benchmarks doivent être affichés aussi** (banc de 200 robots, matrice
   de confusion, précision/rappel/F1, latences, simulation différentielle R12).
   Pas en tableau : en objet visuel.

### 2.3 Le fond

Tout doit rester **visible** : coffre on-chain, plafonds, clés, effets, règles
R1→R12, verdict, certificat, chaîne, banc, journal, surfaces CLI/HTTP/MCP.
L'enjeu n'est pas de réduire le périmètre, c'est de le rendre **lisible sans
lire**.

---

## 3. Inventaire fonctionnel (vérifié dans le code, à ne pas réinventer)

Racine : `HACKATONS/2026-11-15-near/Security/dry-run/`

### 3.1 Le parcours d'une demande

`Gatekeeper::propose` (`crates/dryrun-gate/src/gate.rs:130`) :
1. l'agent propose une action + une **intention déclarée** (`summary`,
   `beneficiaries`, `min_in`, `max_out`) — l'intention ne fait que **restreindre** ;
2. un **bac à sable neuf** est démarré à chaque demande (`near-sandbox`, ≈ 0,55 s) ;
3. la **liste de surveillance** est calculée (coffre, cible, destinataire,
   bénéficiaires, `bob/mallory/nft-holder/owner`, jeton de `min_in`) ;
4. l'état est copié par **enregistrements genesis** (`view_account`, clés
   d'accès, code, stockage) puis injecté en **un seul `patch_state`** avec une
   clé plein accès jetable ;
5. la transaction est **jouée en tant que le coffre** (même compte, mêmes soldes) ;
6. **diff avant/après** → `Effects` ;
7. `judge()` → `ALLOW` / `WARN` / `BLOCK` (le pire verdict gagne).
   `ALLOW` → certificat signé + exécution. `BLOCK` → rien n'est signé.
   `WARN` → confirmation du propriétaire.

### 3.2 Les effets capturés (`crates/dryrun-core/src/types.rs:172`)

deltas NEAR par compte · deltas de jetons NEP-141 · événements NFT
`transfer`/`approve` (parsés depuis les logs NEP-171) · clés ajoutées/retirées
(la clé jetable du simulateur est filtrée) · code déployé/modifié · reçus
(succès/échec/logs) · `stranded` (reçu en échec **après** un gain par un tiers) ·
gas brûlé · delta de stockage (× 10¹⁹ yocto/octet).

### 3.3 Les règles R1→R12 (`crates/dryrun-core/src/rules.rs`)

| | condition | verdict |
|---|---|---|
| R1 | sortie > plafond tx (2 NEAR) ou > reste 24 h (10 NEAR) | BLOCK |
| R2 | clé d'accès ajoutée **sur le coffre** | BLOCK |
| R3 | approbation NFT vers un compte hors liste | BLOCK |
| R4 | `ft_transfer_call` vers contrat inconnu | BLOCK |
| R5 | premier contact avec un contrat (supprimée si R4 mord) | WARN |
| R6 | empreinte du code changée depuis l'accord | BLOCK |
| R7 | un bénéficiaire non déclaré reçoit quelque chose | BLOCK |
| R8 | moins reçu que le minimum déclaré, ou rien | BLOCK |
| R9 | reçu en échec après sortie de fonds | BLOCK |
| R10 | gas > 100 Tgas ou stockage > 10 NEAR | WARN |
| R11 | plus de 8 comptes touchés | BLOCK |
| R12 | divergence différentielle (hors `judge`, ajoutée par le sim) | BLOCK |

### 3.4 Simulation différentielle (R12)

`crates/dryrun-sim/src/differential.rs` : la même demande est rejouée au
**centième** (`DIFFERENTIAL_DIVISOR = 100`). Taux reçu/dépensé comparé ;
tolérance **100 ‰**. Divergent **ou** asymétrique (un seul des deux passages
reçoit quelque chose) → BLOCK. Exemple : `good-dex` 980 ‰ → 975 ‰ (cohérent) ;
`shapeshifter` 980 ‰ → 0 ‰ (divergent).

### 3.5 Certificat Ed25519

`Cert` = `call_digest` (SHA-256 du JSON canonique de l'action), `snapshot_height`,
`expires_height` (**+20 blocs**), `max_out`, `post` (post-conditions), `nonce`
(anti-rejeu). Signé Ed25519 par le simulateur, clé publique déposée on-chain via
`set_sim_key`. Le coffre vérifie **dans l'ordre** : présence (`ERR_NO_CERT`) →
fraîcheur (`ERR_CERT_STALE`) → nonce (`ERR_CERT_USED`) → digest
(`ERR_CERT_MISMATCH`) → `max_out` (`ERR_OVER_TX_CAP`) → clé publique puis
signature (`ERR_BAD_SIGNATURE`).

### 3.6 Le contrat coffre (`contracts/vault/src/`)

- **Méthodes** : `execute` (clé d'agent uniquement), `approve_contract`,
  `forget_contract`, `freeze` (owner **ou** clé de gel), `unfreeze` (owner,
  1 yocto, après 1 h), `set_caps` (resserrage immédiat, desserrage après 1 h),
  `set_sim_key`. Toute action owner exige **1 yoctoNEAR**.
- **Vues** : `status` (13 champs), `caps`, `spent_24h`, `known`, `is_hostile`,
  `sim_key`, `check`, `log(from, limit)`.
- **Fenêtre 24 h** : 24 cases horaires, seules les cases fraîches comptent,
  `rollback` si le reçu échoue.
- **Post-conditions** : `on_done` annule la dépense comptée ; si un **appel**
  avec fonds échoue → **gel + marquage hostile**. `on_post_check` : `min_in`
  non tenu → gel + marquage hostile.
- **13 codes d'erreur** : `ERR_FROZEN`, `ERR_OVER_TX_CAP`, `ERR_OVER_DAILY_CAP`,
  `ERR_UNKNOWN_CONTRACT`, `ERR_HOSTILE`, `ERR_NO_CERT`, `ERR_CERT_STALE`,
  `ERR_CERT_MISMATCH`, `ERR_CERT_USED`, `ERR_BAD_SIGNATURE`, `ERR_COOLDOWN`,
  `ERR_UNAUTHORIZED`, `ERR_ONE_YOCTO`.
- **7 événements NEP-297** : `executed`, `refused`, `frozen`, `unfrozen`,
  `contract_approved`, `postcondition_failed`, `hostile_marked`.
- **Clés** : clé d'agent `FunctionCall::execute` (allocation 1 NEAR), clé de gel
  `FunctionCall::freeze`, **aucune FullAccess** après installation. Journal en
  anneau de 100 entrées.

### 3.7 Les surfaces

- **CLI** : `propose`, `approve`, `freeze`, `status`, `log`, `demo`, `attack`,
  `sim [seed]`, `serve`, `mcp`, `help` ; flag global `--sim auto|real|stub`.
- **HTTP** (`crates/dryrun-gate/src/http.rs`, serveur maison sans framework) :
  `GET /api/status`, `GET /api/log`, `POST /api/propose`, `POST /api/freeze`,
  fichiers statiques, requêtes ≤ 1 Mio.
- **MCP** (JSON-RPC sur stdio) : `wallet_status`, `wallet_propose`,
  `wallet_explain`, `wallet_freeze`.
- **Web** : page statique, zéro CDN, montants en BigInt yoctoNEAR, refresh 3 s.
- **Make** : `build`, `test`, `lint`, `demo`, `attack`, `sim`, `mcp`,
  `demo-testnet`, `clean`, `fmt`, `help`.

### 3.8 Les preuves

- **6 pièges** : `drainer` (R8), `skimmer` (R7+R8), `half-step` (R9),
  `liar` (R8), `approver` (R3), `shapeshifter` (R6) + **vol de clé d'agent**
  rejeté on-chain (`ERR_OVER_TX_CAP`, puis `ERR_OVER_DAILY_CAP` à la 6ᵉ tentative).
- **Banc 200 robots** (`make sim`, graine 42) : 45 transferts honnêtes,
  40 swaps honnêtes, 15 paiements honnêtes / 10 injections, 12 dépassements 24 h,
  12 drainer, 11 skimmer, 11 half-step, 11 liar, 10 approbations NFT,
  9 contrats inconnus, 6 codes changés, 4 poses de clé, 4 gas bombs.
  Sortie : TP, TN, FP, FN, précision, rappel, F1, latences médiane et p95,
  `bench_matrix.json`.
- **7 invariants** vérifiés en fin de `make demo`.
- **Démo 100 % locale**, sans secret ni réseau, < 2 min.

### 3.9 Ce qui n'existe PAS (ne pas l'afficher)

- Aucune boucle de reformulation dans le pare-feu (voir §2.2 point 8).
- Pas de mode `--real` au banc malgré le commentaire du code.
- `make attack` ne joue **pas** la passe « sans pare-feu » (le help du Makefile
  est inexact).
- Pas d'invariant I6 côté contrat (il n'existe que dans la démo du gate).
- Un seul jeton suivi par simulation ; détection NFT **uniquement** par les logs.

---

## 4. Contraintes techniques

- Page **statique**, servie par `dryrun-gate` : **zéro CDN, zéro dépendance
  externe, zéro réseau, zéro image** (tout en CSS/SVG/canvas).
- Doit s'ouvrir en double-clic sur un fichier local.
- Montants en **yoctoNEAR** (BigInt), jamais en flottant.
- Lisible en vidéo (la démo sera filmée) et sur un post X.
- Le code du repo est en **lecture seule** : Claude ne touche qu'à `G-DESIGN/`.

---

## 5. Ce qui est attendu de Claude

1. **3 pistes courtes d'abord** (une phrase + une image mentale chacune), puis
   attendre le choix. Pas de `/planches 6` : peu de pistes, très fortes.
2. Pour la piste choisie : un livrable **ouvrable**, avec le bouton qui joue
   tout le processus.
3. Mettre à jour `DESIGN.md` avec une **nouvelle version datée** (V6), sans
   écraser l'historique, + changelog.
4. Terminer par : résumé + prompt relais + rappel des 6 commandes.

---

## 6. Ce qui a raté avant (à ne pas refaire)

- **V1/V2 (`planches/`)** : six tableaux de bord documentés. Trop de texte,
  aucune identité visuelle, rien de marquant. C'était de la fiche produit, pas
  du design.
- **V3 (`planches-v3/` : signal, orbit, strip)** : meilleure identité (2 couleurs,
  fond sombre) mais **un bout visuel et le reste en texte**. Des panneaux 2D
  alignés dans une grille, avec une petite animation au milieu.
- **V4 (`planches-v3/pipeline.html`)** : le bon récit (les deux agents, le bac à
  sable, la chaîne) mais toujours en **colonnes de texte** : le « visuel » était
  une ligne de points et des barres. Pas un espace.
- **V5 (`planches-3d/` : dix mondes)** : la page était bien un espace 3D, mais
  **13 panneaux de verre flottants trop petits**, scène encombrée, panneaux qui
  se chevauchent, texte illisible à l'échelle, aucune hiérarchie : on ne
  comprenait **rien d'un coup d'œil**. Dix variantes d'une même mauvaise idée
  au lieu d'une idée forte. Le 3D pour le 3D.

**Leçon à retenir** : la 3D n'est pas une décoration à ajouter sur la même
maquette. Il faut d'abord décider **quelle métaphore spatiale raconte le
produit** (un sas, un poste de contrôle, un tunnel de vérification, une balance,
un couloir de sécurité…), la pousser à fond, et n'y mettre **que** ce qui sert
cette métaphore. Moins d'éléments, plus grands, plus lisibles, plus assumés.
Une seule idée forte par piste.

---

## 7. Repères

- Brief initial : `G-DESIGN/BRIEF-UI.md`
- Prompt d'origine : `G-DESIGN/PROMPT-DESIGN.md`
- Historique design (rejeté, ne pas reprendre) : `G-DESIGN/DESIGN.md` V1→V5
- Repo code : dossier parent (`README.md`, `docs/SPEC.md`, `docs/SPEC_V1_MAX.md`,
  `contracts/INTERFACE.md`)
- Consignes permanentes : `HACKATONS/CLAUDE.md`, `HACKATONS/prompts/07-design.md`
- Deadline design : 14/11 · soumission 15/11 00:59 Paris
