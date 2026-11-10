//! Objets du domaine : demande, intention, effets, verdict.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// YoctoNEAR.
pub type Yocto = u128;

/// Ce que l'agent demande au coffre.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Demande {
    /// Transfert direct de NEAR.
    Transfer { to: String, amount: Yocto },
    /// Appel de fonction sur un contrat, avec dépôt joint.
    FunctionCall {
        contract: String,
        method: String,
        #[serde(default)]
        args: serde_json::Value,
        #[serde(default)]
        deposit: Yocto,
        #[serde(default = "default_gas")]
        gas: u64,
    },
}

fn default_gas() -> u64 {
    30_000_000_000_000
}

impl Demande {
    /// Montant de NEAR attaché par la demande (dépôt ou montant transféré).
    pub fn amount(&self) -> Yocto {
        match self {
            Demande::Transfer { amount, .. } => *amount,
            Demande::FunctionCall { deposit, .. } => *deposit,
        }
    }

    /// Compte appelé par la demande, s'il y en a un.
    pub fn target(&self) -> Option<&str> {
        match self {
            Demande::Transfer { .. } => None,
            Demande::FunctionCall { contract, .. } => Some(contract),
        }
    }

    /// Résumé lisible, en une ligne.
    pub fn summary(&self) -> String {
        match self {
            Demande::Transfer { to, amount } => {
                format!("transfer {} yoctoNEAR to {}", amount, to)
            }
            Demande::FunctionCall {
                contract,
                method,
                deposit,
                ..
            } => format!("call {}.{} with {} yoctoNEAR", contract, method, deposit),
        }
    }
}

/// L'intention déclarée par l'agent : un contrat avec soi-même.
/// Elle ne sert qu'à **restreindre** : mentir ne donne que des refus.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Intent {
    /// Phrase lisible : « payer 1 NEAR à bob ».
    #[serde(default)]
    pub summary: String,
    /// Comptes qui ont le droit de recevoir quelque chose.
    #[serde(default)]
    pub beneficiaries: Vec<String>,
    /// Jetons et montants minimaux attendus : `(jeton, minimum)`.
    #[serde(default)]
    pub min_in: Vec<(String, Yocto)>,
    /// Plafond de sortie annoncé par l'agent, s'il en déclare un.
    #[serde(default)]
    pub max_out: Option<(String, Yocto)>,
}

impl Intent {
    /// Une intention sans bénéficiaire ni minimum est un simple transfert.
    pub fn is_plain_transfer(&self) -> bool {
        self.beneficiaries.is_empty() && self.min_in.is_empty()
    }

    /// Le minimum déclaré pour un jeton donné.
    pub fn min_in_of(&self, token: &str) -> Option<Yocto> {
        self.min_in
            .iter()
            .find(|(t, _)| t == token)
            .map(|(_, v)| *v)
    }
}

/// Instantané de l'état lu avant la simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Snapshot {
    pub height: u64,
    pub block_hash: String,
    pub accounts: Vec<AccountSnap>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountSnap {
    pub id: String,
    /// Empreinte du code du compte (vide pour un compte sans contrat).
    pub code_hash: String,
}

/// Variation de solde NEAR d'un compte.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NearDelta {
    pub account: String,
    pub delta: i128,
}

/// Variation de solde d'un jeton fongible.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FtDelta {
    pub token: String,
    pub account: String,
    pub delta: i128,
}

/// Événement NFT observé pendant la simulation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NftEvent {
    Transfer {
        collection: String,
        token: String,
        to: String,
    },
    Approve {
        collection: String,
        token: String,
        to: String,
    },
}

/// Changement de clé d'accès observé.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KeyEvent {
    Added { account: String, scope: String },
    Removed { account: String, scope: String },
}

/// Changement de code observé.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CodeEvent {
    Deployed { account: String },
    Changed { account: String },
}

/// Un reçu observé pendant la simulation.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReceiptInfo {
    pub id: String,
    /// `success`, `failed` ou `unknown`.
    pub status: String,
    pub receiver: String,
    #[serde(default)]
    pub logs: Vec<String>,
}

/// Ce que la transaction a réellement fait dans le bac à sable.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Effects {
    pub snapshot: Snapshot,
    pub near_delta: Vec<NearDelta>,
    pub ft_delta: Vec<FtDelta>,
    pub nft: Vec<NftEvent>,
    pub keys: Vec<KeyEvent>,
    pub code: Vec<CodeEvent>,
    pub receipts: Vec<ReceiptInfo>,
    /// Vrai si un reçu a échoué après une sortie de fonds.
    pub stranded: bool,
    pub gas_burnt: u64,
    pub storage_delta: u128,
}

impl Effects {
    /// Variation NEAR d'un compte.
    pub fn near_delta_of(&self, account: &str) -> i128 {
        self.near_delta
            .iter()
            .find(|d| d.account == account)
            .map(|d| d.delta)
            .unwrap_or(0)
    }

    /// Variation d'un jeton fongible.
    pub fn ft_delta_of(&self, token: &str, account: &str) -> i128 {
        self.ft_delta
            .iter()
            .find(|d| d.token == token && d.account == account)
            .map(|d| d.delta)
            .unwrap_or(0)
    }

    /// Comptes dont le solde a bougé.
    pub fn touched_accounts(&self) -> Vec<String> {
        let mut out: Vec<String> = self
            .near_delta
            .iter()
            .filter(|d| d.delta != 0)
            .map(|d| d.account.clone())
            .collect();
        for d in &self.ft_delta {
            if d.delta != 0 && !out.contains(&d.account) {
                out.push(d.account.clone());
            }
        }
        out
    }

    /// Les effets contiennent-ils une sortie de fonds ?
    pub fn has_outflow(&self) -> bool {
        self.near_delta.iter().any(|d| d.delta < 0) || self.ft_delta.iter().any(|d| d.delta < 0)
    }
}

/// Verdict du pare-feu.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Verdict {
    Allow,
    Warn,
    Block,
}

impl Verdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Verdict::Allow => "ALLOW",
            Verdict::Warn => "WARN",
            Verdict::Block => "BLOCK",
        }
    }

    /// Le verdict est-il au moins aussi grave que `other` ? (Allow < Warn < Block)
    pub fn is_at_least(&self, other: Verdict) -> bool {
        self.severity() >= other.severity()
    }

    /// Gravité croissante.
    pub fn severity(&self) -> u8 {
        match self {
            Verdict::Allow => 0,
            Verdict::Warn => 1,
            Verdict::Block => 2,
        }
    }
}

/// Une règle qui a mordu.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleHit {
    /// Identifiant de la règle, par exemple `R1`.
    pub id: String,
    /// Titre court de la règle.
    pub title: String,
    /// Détail chiffré, lisible par un humain.
    pub detail: String,
    /// Gravité de la règle.
    pub severity: Verdict,
}

/// Décision complète : verdict, règles en cause et effets observés.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Decision {
    pub verdict: Verdict,
    pub rules: Vec<RuleHit>,
    pub effects: Effects,
}

impl Decision {
    pub fn allow(effects: Effects) -> Self {
        Decision {
            verdict: Verdict::Allow,
            rules: vec![],
            effects,
        }
    }

    /// Identifiants des règles en cause, dans l'ordre.
    pub fn rule_ids(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.id.clone()).collect()
    }
}

/// Contrat connu du propriétaire : compte → empreinte du code à l'accord.
pub type KnownContracts = BTreeMap<String, String>;
