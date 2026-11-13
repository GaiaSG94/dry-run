//! Langue des réponses du serveur MCP : français ou anglais.
//!
//! L'anglais est le défaut (jury, vidéo, agents IA) ; le français reste
//! disponible avec `--lang fr`. Le contenu jugé par le pare-feu (verdict,
//! règles, montants) ne change pas : seule la **manière de le dire** change.

use serde_json::{json, Value};

use crate::cardfmt::CardView;
use crate::sim_types::VaultStatus;

/// Langue d'une réponse MCP.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Lang {
    /// Anglais (défaut).
    #[default]
    En,
    /// Français.
    Fr,
}

/// Description anglaise des règles R1 à R12.
const RULES_EN: &[(&str, &str)] = &[
    (
        "R1",
        "output above the per-transaction cap or the remaining daily cap",
    ),
    ("R2", "an access key appears on one of our accounts"),
    (
        "R3",
        "NFT or multi-token approval to an account outside the list",
    ),
    (
        "R4",
        "transfer with call (ft_transfer_call) to an unknown contract",
    ),
    ("R5", "first contact with a contract"),
    ("R6", "known contract whose code hash changed"),
    (
        "R7",
        "a beneficiary not named in the intent received something",
    ),
    ("R8", "received less than the declared minimum, or nothing"),
    ("R9", "a receipt failed after the funds left"),
    ("R10", "gas burnt or storage deposit above a threshold"),
    ("R11", "more than eight accounts touched"),
    (
        "R12",
        "divergent differential simulation (two-faced contract)",
    ),
];

impl Lang {
    /// Analyse l'option `--lang`.
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        match text.trim().to_lowercase().as_str() {
            "en" | "english" | "anglais" => Ok(Lang::En),
            "fr" | "french" | "français" | "francais" => Ok(Lang::Fr),
            other => anyhow::bail!("langue inconnue : {other} (attendues : en, fr)"),
        }
    }

    /// Séparateur de label (le français garde l'espace avant le deux-points).
    fn sep(&self) -> &'static str {
        match self {
            Lang::En => ": ",
            Lang::Fr => " : ",
        }
    }

    /// Ligne du verdict.
    pub fn verdict_line(&self, verdict: &str) -> String {
        format!("VERDICT{sep}{verdict}", sep = self.sep())
    }

    /// Ligne « exécuté sur » : honnête sur le lieu réel de l'exécution.
    ///
    /// Le coffre local est une simulation ; seule la chaîne publique porte
    /// une empreinte de transaction cliquable.
    pub fn executed_line(&self, executed: bool, on_chain: bool, tx_hash: Option<&str>) -> String {
        let label = match self {
            Lang::En => "Executed on",
            Lang::Fr => "Exécuté sur",
        };
        if !executed {
            let none = match self {
                Lang::En => "nothing (rejected before signing)",
                Lang::Fr => "rien (refusé avant signature)",
            };
            return format!("{label}{sep}{none}", sep = self.sep());
        }
        if on_chain {
            if let Some(hash) = tx_hash {
                return format!("{label}{sep}NEAR testnet · tx {hash}", sep = self.sep());
            }
            return format!("{label}{sep}NEAR testnet", sep = self.sep());
        }
        let local = match self {
            Lang::En => "the local vault (simulation)",
            Lang::Fr => "le coffre local (simulation)",
        };
        format!("{label}{sep}{local}", sep = self.sep())
    }

    /// Ligne des règles en cause.
    pub fn rules_line(&self, rules: &[crate::cardfmt::RuleHitView]) -> String {
        let label = match self {
            Lang::En => "Rules triggered",
            Lang::Fr => "Règles en cause",
        };
        if rules.is_empty() {
            let none = match self {
                Lang::En => "none",
                Lang::Fr => "aucune",
            };
            return format!("{label}{sep}{none}", sep = self.sep());
        }
        let ids: Vec<String> = rules
            .iter()
            .map(|rule| match self {
                Lang::En => format!("{} ({})", rule.id, en_label(&rule.id)),
                Lang::Fr => format!("{} ({})", rule.id, rule.title),
            })
            .collect();
        format!("{label}{sep}{}", ids.join(", "), sep = self.sep())
    }

    /// Titre de la carte des effets.
    pub fn card_title(&self) -> &'static str {
        match self {
            Lang::En => "EFFECTS CARD (simulated):",
            Lang::Fr => "CARTE DES EFFETS SIMULÉS :",
        }
    }

    /// Carte des effets, dans la langue demandée.
    ///
    /// Le français reprend la carte rendue par le cœur (`dryrun_core`) ;
    /// l'anglais est rendu ici, à partir de la même vue structurée.
    pub fn effects_card(&self, card: &CardView) -> String {
        match self {
            Lang::En => card_en(card),
            Lang::Fr => card.rendered_terminal.clone(),
        }
    }

    /// État du coffre, dans la langue demandée.
    pub fn status_text(&self, status: &VaultStatus) -> String {
        match self {
            Lang::En => status_en(status),
            Lang::Fr => status_fr(status),
        }
    }

    /// Confirmation de gel.
    pub fn frozen_text(&self) -> &'static str {
        match self {
            Lang::En => {
                "The vault was frozen in one gesture. Any later transaction will be rejected."
            }
            Lang::Fr => {
                "Le coffre a été gelé d'un geste. Toute transaction ultérieure sera rejetée."
            }
        }
    }

    /// Message d'erreur de gel.
    pub fn frozen_error(&self, error: &str) -> String {
        match self {
            Lang::En => format!("Freeze error: {error}"),
            Lang::Fr => format!("Erreur gel : {error}"),
        }
    }

    /// Explication d'une règle.
    pub fn explain_rule(&self, rule_id: &str) -> String {
        match self {
            Lang::En => match RULES_EN.iter().find(|(id, _)| *id == rule_id) {
                Some((id, text)) => format!("Rule {id}: {text}"),
                None => format!("Unknown rule: {rule_id}. Valid rules are R1 to R12."),
            },
            Lang::Fr => {
                for (id, desc) in dryrun_core::RULES {
                    if *id == rule_id {
                        return format!("Règle {id} : {desc}");
                    }
                }
                format!("Règle inconnue : {rule_id}. Les règles valides sont R1 à R12.")
            }
        }
    }

    /// Rejet d'une demande mal formée.
    pub fn rejected(&self, message: &str) -> String {
        match self {
            Lang::En => format!("REJECTED (malformed request): {message}"),
            Lang::Fr => format!("REJET (demande mal formée) : {message}"),
        }
    }

    /// Erreur de simulation.
    pub fn simulation_error(&self, error: &str) -> String {
        match self {
            Lang::En => format!("Simulation error: {error}"),
            Lang::Fr => format!("Erreur simulation : {error}"),
        }
    }

    /// Outil inconnu.
    pub fn unknown_tool(&self, name: &str) -> String {
        match self {
            Lang::En => format!("Unknown tool: {name}"),
            Lang::Fr => format!("Outil inconnu : {name}"),
        }
    }

    /// Champ inconnu dans les arguments d'un outil.
    pub fn unknown_field(&self, name: &str, accepted: &[&str]) -> String {
        match self {
            Lang::En => format!(
                "unknown field `{name}` — accepted fields: {}",
                accepted.join(", ")
            ),
            Lang::Fr => format!(
                "champ inconnu `{name}` — champs acceptés : {}",
                accepted.join(", ")
            ),
        }
    }

    /// Champ obligatoire manquant.
    pub fn missing_field(&self, name: &str) -> String {
        match self {
            Lang::En => format!("missing `{name}` field"),
            Lang::Fr => format!("champ `{name}` manquant"),
        }
    }

    /// Champ obligatoire manquant, avec la raison.
    pub fn missing_field_why(&self, name: &str, why: &str) -> String {
        match self {
            Lang::En => format!("missing `{name}` field: {why}"),
            Lang::Fr => format!("champ `{name}` manquant : {why}"),
        }
    }

    /// Montant illisible.
    pub fn invalid_amount(&self, value: &str) -> String {
        match self {
            Lang::En => format!("invalid amount `{value}`: expected a decimal NEAR amount"),
            Lang::Fr => format!("montant invalide `{value}` : montant NEAR décimal attendu"),
        }
    }

    /// Valeur d'énumération inconnue.
    pub fn unknown_kind(&self, value: &str) -> String {
        match self {
            Lang::En => format!("unknown kind `{value}` — expected `transfer` or `function_call`"),
            Lang::Fr => {
                format!("kind inconnu `{value}` — attendu `transfer` ou `function_call`")
            }
        }
    }

    /// Outils MCP, décrits dans la langue demandée.
    pub fn tools(&self) -> Vec<Value> {
        match self {
            Lang::En => tools_en(),
            Lang::Fr => tools_fr(),
        }
    }
}

/// Libellé anglais d'une règle, à partir de son identifiant.
fn en_label(rule_id: &str) -> &'static str {
    RULES_EN
        .iter()
        .find(|(id, _)| *id == rule_id)
        .map(|(_, text)| *text)
        .unwrap_or("rule triggered")
}

/// Carte des effets en anglais, rendue depuis la vue structurée.
fn card_en(card: &CardView) -> String {
    let mut out = String::new();
    out.push_str(&format!(" {}\n", card.summary));
    out.push_str(&format!(" {:<70} {}\n", "", card.verdict_str));
    out.push('\n');
    out.push_str(" balances\n");
    if card.near_deltas.is_empty() && card.ft_deltas.is_empty() {
        out.push_str("   (no movement)\n");
    }
    for delta in &card.near_deltas {
        out.push_str(&format!(
            "   {:<40} {:>+} NEAR\n",
            delta.account, delta.formatted
        ));
    }
    for delta in &card.ft_deltas {
        out.push_str(&format!(
            "   {:<40} {:>+} {}\n",
            delta.account, delta.delta, delta.token
        ));
    }
    out.push_str(" receipts\n");
    out.push_str(&format!(
        "   {}, {} failed\n",
        card.receipts.total, card.receipts.failed
    ));
    out.push_str(" rules\n");
    if card.rules.is_empty() {
        out.push_str("   none\n");
    } else {
        for rule in &card.rules {
            out.push_str(&format!("   {} {}\n", rule.id, en_label(&rule.id)));
        }
    }
    out.push_str(&format!(
        "\n simulated at block {} — gas {} — {}\n",
        card.height,
        card.gas_burnt,
        if card.receipts.stranded {
            "funds stranded"
        } else {
            "no failed receipt"
        }
    ));
    out
}

/// État du coffre en français.
fn status_fr(status: &VaultStatus) -> String {
    format!(
        "Compte coffre : {}\nSolde actuel  : {} NEAR\nPlafond par tx : {} NEAR\nPlafond 24h   : {} NEAR\nDépensé 24h   : {} NEAR\nStatut        : {}\nContrats connus : {:?}",
        status.account_id,
        crate::Amount(status.balance.yocto()).near(),
        crate::Amount(status.tx_cap.yocto()).near(),
        crate::Amount(status.daily_cap.yocto()).near(),
        crate::Amount(status.spent_24h.yocto()).near(),
        if status.is_frozen { "GELÉ (FROZEN)" } else { "OUVERT (OPEN)" },
        status.known_contracts.keys().collect::<Vec<_>>()
    )
}

/// État du coffre en anglais.
fn status_en(status: &VaultStatus) -> String {
    format!(
        "Vault account : {}\nBalance       : {} NEAR\nPer-tx cap    : {} NEAR\n24h cap       : {} NEAR\nSpent 24h     : {} NEAR\nState         : {}\nKnown contracts : {:?}",
        status.account_id,
        crate::Amount(status.balance.yocto()).near(),
        crate::Amount(status.tx_cap.yocto()).near(),
        crate::Amount(status.daily_cap.yocto()).near(),
        crate::Amount(status.spent_24h.yocto()).near(),
        if status.is_frozen { "FROZEN" } else { "OPEN" },
        status.known_contracts.keys().collect::<Vec<_>>()
    )
}

/// Outils MCP, décrits en anglais.
fn tools_en() -> Vec<Value> {
    vec![
        json!({
            "name": "wallet_status",
            "description": "Read the state of the Dry Run vault: balance, rolling 24h caps, approved contracts, freeze state.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "wallet_propose",
            "description": "Propose a financial action (transfer or contract call). It is SIMULATED before any signature. The verdict (ALLOW, WARN, BLOCK) and the effects card are returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["transfer", "function_call"] },
                    "to": { "type": "string", "description": "Recipient of the transfer" },
                    "amount_near": { "type": "string", "description": "Amount in decimal NEAR (e.g. '1.5')" },
                    "contract": { "type": "string", "description": "Contract targeted by the call" },
                    "method": { "type": "string", "description": "Method called" },
                    "args": { "type": "object", "description": "JSON arguments" },
                    "deposit_near": { "type": "string", "description": "Attached deposit in NEAR (e.g. '2.0')" },
                    "intent": { "type": "string", "description": "Declared intent in plain language (e.g. 'pay 1 NEAR to bob')" }
                },
                "required": ["intent"]
            }
        }),
        json!({
            "name": "wallet_freeze",
            "description": "Freeze the vault immediately with the freeze key. Every later transaction is rejected.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "wallet_explain",
            "description": "Get the detailed explanation of a Dry Run firewall rule (R1 to R12).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rule_id": { "type": "string", "description": "Rule identifier (e.g. 'R1', 'R7', 'R8')" }
                },
                "required": ["rule_id"]
            }
        }),
    ]
}

/// Outils MCP, décrits en français.
fn tools_fr() -> Vec<Value> {
    vec![
        json!({
            "name": "wallet_status",
            "description": "Consulter l'état du wallet coffre : soldes, plafonds glissants 24h, contrats approuvés, statut de gel.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "wallet_propose",
            "description": "Proposer une action financière (transfert ou appel de contrat). Elle est SIMULÉE dans un bac à sable local avant toute signature. Le verdict (ALLOW, WARN, BLOCK) et la carte des effets sont retournés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["transfer", "function_call"] },
                    "to": { "type": "string", "description": "Destinataire du transfert" },
                    "amount_near": { "type": "string", "description": "Montant en NEAR décimal (ex: '1.5')" },
                    "contract": { "type": "string", "description": "Contrat visé par l'appel" },
                    "method": { "type": "string", "description": "Méthode appelée" },
                    "args": { "type": "object", "description": "Arguments JSON" },
                    "deposit_near": { "type": "string", "description": "Dépôt joint en NEAR (ex: '2.0')" },
                    "intent": { "type": "string", "description": "Intention déclarée en langage clair (ex: 'payer 1 NEAR à bob')" }
                },
                "required": ["intent"]
            }
        }),
        json!({
            "name": "wallet_freeze",
            "description": "Geler immédiatement le coffre d'un geste d'urgence via la clé de gel. Bloque toute sortie de fonds ultérieure.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "wallet_explain",
            "description": "Obtenir l'explication détaillée d'une règle du pare-feu Dry Run (R1 à R12).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rule_id": { "type": "string", "description": "Identifiant de la règle (ex: 'R1', 'R7', 'R8')" }
                },
                "required": ["rule_id"]
            }
        }),
    ]
}
