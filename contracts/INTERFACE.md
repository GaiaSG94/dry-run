# Contrats de démonstration — signatures et prérequis

Fiche d'interface pour `dryrun-sim` et `dryrun-gate`. Tout est near-sdk 5.30,
`crate-type = ["cdylib", "rlib"]`, contrats 100 % JSON (pas de Borsh sur l'état).

## Noms de méthodes appelées par le coffre (`VaultAction::Call`)

| Compte | Méthode | Args JSON | Dépôt | Retour |
|---|---|---|---|---|
| `test-token.test.near` | `ft_transfer` | `{"receiver_id","amount"}` (U128 en chaîne) | 1 yocto | — |
| `test-token.test.near` | `ft_transfer_call` | `{"receiver_id","amount","msg"}` | 1 yocto | `U128` |
| `test-token.test.near` | `mint_to` | `{"account_id","amount"}` | 0 | — |
| `test-nft.test.near` | `nft_approve` | `{"to","token_id"}` | 1 yocto | — |
| `test-nft.test.near` | `nft_transfer` | `{"to","token_id"}` | 1 yocto | — |
| `good-dex.test.near` | `swap` | `{"token_out":"TEST","min_out":"<u128>"}` | ≥ 2 NEAR | `bool` |
| tous les pièges | `trap` | `{"args":"<texte libre>"}` | 2 NEAR | `bool` |

## Initialisation (`#[init] new(...)`)

- `test-token.new { owner_id: "owner.test.near" }`
- `test-nft.new { owner_id: "owner.test.near" }`
- `good-dex.new { owner_id: "owner.test.near", token_id: "test-token.test.near" }`
- `trap-drainer`, `trap-half-step`, `trap-liar` : pas d'`init` (état `Default`).
- `trap-skimmer.new { owner_id, token_id, skim_to: "mallory.test.near" }`
- `trap-approver.new { collection: "test-nft.test.near", thief: "mallory.test.near" }`
- `trap-shapeshifter.new { owner_id, token_id }`

## Prérequis de la démo

1. **Jeton TEST** — frappé par `test-token.mint_to` (seul `owner.test.near`, le
   propriétaire du jeton, peut le faire ; le coffre n'a pas ce droit).
2. **Réapprovisionnement de `good-dex`** — `owner.test.near` appelle
   `test-token.ft_transfer_call { receiver_id: "good-dex.test.near", amount, msg: "fund" }`
   (côté DEX, `ft_on_transfer` crédite la réserve). Variante : `ft_transfer` vers le
   DEX puis `good-dex.fund { amount }`. Ne pas combiner les deux.
3. **Réserve des pièges à jetons** — `skimmer` et `shapeshifter` ont une réserve
   interne approvisionnée par leur propriétaire (`fund { amount }`, jetons à
   transférer au préalable sur le compte du contrat).
4. **NFT détenu par le coffre** — `approver` fait approuver `mallory` par le
   détenteur courant : `test-nft` n'autorise `nft_approve` que par le détenteur.
   Donc `owner.test.near` frappe le jeton `"1"` directement au coffre
   (`mint_to { to: "vault.test.near", token_id: "1" }`) ou le lui transfère avant
   l'attaque (`nft_transfer { to: "vault.test.near", token_id: "1" }`).
5. **Remplacement de code (`shapeshifter`)** — `deploy { code: "<base64 ou tableau d'octets>" }`
   est **public** et sans contrôle d'accès ; le démonstrateur y dépose un autre wasm
   pour changer l'empreinte (R6). Le code déployé doit accepter l'état courant
   (mêmes champs JSON), sinon les appels suivants échouent à la désérialisation.

## Règles visées

| Piège | Ce qu'on observe | Règle |
|---|---|---|
| `drainer` | aucun reçu vers le coffre | R8 |
| `skimmer` | 1,6 TEST au coffre, 0,4 TEST à `mallory` | R7 + R8 |
| `half-step` | reçu vers `ghost.test.near` en échec après l'encaissement | R9 |
| `liar` | `true` et un `EVENT_JSON` « swap », zéro reçu | R8 |
| `approver` | `nft_approve` vers `mallory` sur `test-nft` | R3 |
| `shapeshifter` | empreinte du code modifiée après accord | R6 |

Le simulateur peut lire les approbations NFT sur les logs NEP-297 :
`EVENT_JSON:{"standard":"nep171","version":"1.0.0","event":"nft_approve","data":[{"token_id":"1","owner_id":"vault.test.near","approved_account_id":"mallory.test.near"}]}`.
