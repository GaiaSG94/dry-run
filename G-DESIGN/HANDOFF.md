# HANDOFF — dry-run (retours croisés, ajout uniquement)

## [Claude] 2026-11-12 · nuit — déploiement testnet demandé par Gaia

**Constat (lecture du code, rien n'a été modifié) :**

- `scripts/demo-testnet.sh` compile le contrat `vault`, le déploie sur le compte
  `NEAR_ACCOUNT`, l'initialise (plafonds 2 NEAR / tx et 10 NEAR / 24 h) et lit
  `status()`. C'est tout.
- `dryrun-gate` n'a **aucun mode testnet** : pas d'option réseau ni RPC, seulement
  `--sim auto|real|stub`. Après le déploiement, la page, l'API HTTP, la CLI et le
  MCP utilisent toujours le coffre **en mémoire** (`vault.test.near`, 50 NEAR).
- Conséquence : le script prouve que le contrat tourne sur testnet. Il ne prouve
  pas que le pare-feu protège un portefeuille testnet. À ne pas présenter au jury
  comme « le pare-feu tourne sur testnet ».
- Le script donne au coffre son propre compte comme `owner`. La même clé est donc
  à la fois l'agent et le propriétaire. Ça suffit pour une démo, mais c'est à dire.

**Prérequis vérifiés sur le PC de Gaia :**

| Élément | État |
|---|---|
| `cargo` | présent |
| cible `wasm32-unknown-unknown` | installée |
| CLI NEAR (`near`, near-cli-rs) | **absente** du PATH |
| compte testnet approvisionné | **à créer par Gaia** (Claude ne manipule ni compte ni clé) |
| variable `NEAR_ACCOUNT` | à définir (nom du compte uniquement) |
| `bash` + `python3` (le script les utilise) | à vérifier sous Windows |

**Options :**

- **A · Coffre seul sur testnet** : environ 30 min. Gaia crée le compte, opencode
  installe la CLI NEAR, lance le script, puis ajoute au README le nom du compte,
  le lien vers l'explorateur et ce que ça prouve (et ne prouve pas).
- **B · A + mode testnet du pare-feu** : `dryrun-gate` lirait `status()` du coffre
  déployé par RPC et soumettrait les demandes ALLOW au contrat. C'est du code, il
  faut plusieurs heures, et c'est risqué à deux jours de la date limite.
- **C · Rien** : on garde « utilisable en local » et on le dit clairement.

**Recommandation Claude : A maintenant.** B seulement une fois la vidéo, le post X
et le formulaire prêts. Date limite : 14/11, 23:59 UTC.

**Autre constat (design, pour info) :** sur mobile (375 px), la scène 3D est petite,
ses étiquettes se chevauchent et le terminal n'apparaît pas. Le mode réel s'y
connecte bien. Sur ordinateur (1440 px), rien ne se chevauche, donc la retouche
`padT +70` annulée n'est pas nécessaire.

## [Claude] 2026-11-12 · 23 h — option A faite par Claude (exception demandée par Gaia)

- Compte `dryrun-vault-1112.testnet` créé via le faucet (accord de Gaia) ; clé dans le trousseau Windows de la CLI NEAR, jamais affichée.
- CLI : near-cli-rs 0.31.1 installée hors dépôt (dossier temporaire), pas dans le PATH.
- `vault.wasm` existant déployé tel quel (rustc 1.98, accepté par testnet), `new` appelé avec les plafonds du script.
- Vérifié sur la chaîne : `execute` 5 NEAR → `ERR_OVER_TX_CAP` ; 0,1 NEAR → exécuté, `spent_24h` = 0,1 NEAR.
- README : section « 6 bis » + deux lignes du tableau « Limites », sur la branche `docs/testnet-vault` (non commitée).
- Reste (code, opencode) : option B, relier `dryrun-gate` au coffre testnet.
