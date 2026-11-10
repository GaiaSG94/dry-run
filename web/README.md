# Interface web de Dry Run

Page statique servie par le garde-barrière : aucun CDN, aucune dépendance
externe, aucun accès réseau. Tout tourne en local.

```bash
# à la racine du dépôt
cargo run -p dryrun-gate -- serve --port 8787
# puis ouvrir http://localhost:8787
```

Options : `--port <n>` (défaut 8787), `--web-dir <dossier>` (défaut `web`) et
`--sim auto|real|stub` (option globale ; `stub` joue la démonstration sur un
coffre en mémoire, sans bac à sable NEAR).

## Fichiers

| Fichier | Rôle |
|---|---|
| `index.html` | structure de la page : état du coffre, formulaire, carte des effets, frise, bouton de gel |
| `style.css` | thème sombre lisible, verdicts colorés (vert `ALLOW`, jaune `WARN`, rouge `BLOCK`) |
| `app.js` | appels à l'API JSON, rendu de la carte, frise et gel — montants en `BigInt` (yoctoNEAR exacts) |

## API consommée par la page

| Route | Méthode | Corps | Réponse |
|---|---|---|---|
| `/` | GET | — | `index.html` (et les fichiers statiques de `web/`) |
| `/api/status` | GET | — | état du coffre |
| `/api/log` | GET | — | frise des demandes (tableau) |
| `/api/propose` | POST | `{ kind, ..., intent, confirm? }` | `Outcome` : `verdict`, `rules`, `card`, `card_view`, `executed`, `note` |
| `/api/freeze` | POST | — | état du coffre gelé |

Exemple de proposition :

```json
{
  "kind": "transfer",
  "to": "bob.test.near",
  "amount": "1000000000000000000000000",
  "intent": { "summary": "payer 1 NEAR à bob", "beneficiaries": ["bob.test.near"], "min_in": [] }
}
```

Les montants sont des **chaînes décimales en yoctoNEAR** : un `f64` JavaScript
perdrait la précision au-delà de 2^53, donc aucun nombre flottant n'est utilisé.

## Limite importante : qui détient quelle clé

Le garde-barrière ne détient **que la clé limitée de l'agent** (`FunctionCall`
sur `execute`). Il ne peut donc **ni** changer les plafonds, **ni** approuver un
contrat, **ni** dégeler le coffre.

Un verdict `WARN` (par exemple « premier contact avec un contrat », règle R5)
demande une décision du **propriétaire** :

- dans la **démo locale**, la clé du propriétaire vit dans le même processus
  (la CLI et le serveur de démonstration peuvent appeler `approve_contract`) ;
  c'est un raccourci assumé pour la vidéo et le jury ;
- en **production**, cette étape se fait depuis l'appareil du propriétaire :
  le simulateur ne fait que proposer, le propriétaire signe l'approbation, puis
  la simulation est rejouée et la clé de l'agent signe l'exécution.

Le bouton de gel est le seul geste qui reste utilisable quand tout le reste est
refusé : la clé de gel ne peut **que** geler (invariant I7), elle ne peut rien
dégeler ni dépenser.
