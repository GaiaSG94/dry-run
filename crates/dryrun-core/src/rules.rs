//! Moteur de règles : le verdict vient des **effets**, jamais du texte de l'agent.
//!
//! Chaque règle observe le rapport d'effets produit par la simulation et
//! le compare au contexte (intention déclarée, contrats connus, plafonds).

use crate::types::{
    Decision, Demande, Effects, Intent, KeyEvent, KnownContracts, NftEvent, RuleHit, Verdict, Yocto,
};

/// Le tableau des règles, tel qu'il figure dans le cahier des charges.
pub const RULES: &[(&str, &str)] = &[
    ("R1", "sortie au-dessus du plafond par transaction ou du reste du jour"),
    ("R2", "une clé d'accès apparaît sur l'un de nos comptes"),
    ("R3", "approbation de NFT ou de multi-jeton vers un compte hors liste"),
    ("R4", "transfert avec appel (ft_transfer_call) vers un contrat inconnu"),
    ("R5", "premier contact avec un contrat"),
    ("R6", "contrat connu dont l'empreinte du code a changé"),
    ("R7", "un bénéficiaire que l'intention ne nomme pas reçoit quelque chose"),
    ("R8", "reçu moins que le minimum déclaré, ou rien"),
    ("R9", "un reçu échoue après une sortie de fonds"),
    ("R10", "gas brûlé ou dépôt de stockage au-dessus d'un seuil"),
    ("R11", "plus de huit comptes touchés"),
];

/// Contexte de jugement.
#[derive(Clone, Debug)]
pub struct RuleCtx {
    /// Compte du coffre.
    pub vault: String,
    /// La demande telle que proposée.
    pub demand: Demande,
    /// L'intention déclarée par l'agent.
    pub intent: Intent,
    /// Contrats connus : compte → empreinte du code à l'accord.
    pub known: KnownContracts,
    /// Empreinte **actuelle** du code des comptes chargés.
    pub current_code: KnownContracts,
    /// Plafond par transaction, en yoctoNEAR.
    pub tx_cap: Yocto,
    /// Plafond sur 24 heures glissantes, en yoctoNEAR.
    pub daily_cap: Yocto,
    /// Déjà dépensé sur la fenêtre glissante, en yoctoNEAR.
    pub daily_spent: Yocto,
    /// Comptes qui touchent les frais et ne sont jamais des bénéficiaires.
    pub fee_accounts: Vec<String>,
    /// Seuil de gas brûlé au-delà duquel on avertit.
    pub gas_threshold: u64,
    /// Seuil de dépôt de stockage au-delà duquel on avertit.
    pub storage_threshold: Yocto,
    /// Nombre maximum de comptes touchés.
    pub max_touched: usize,
}

impl Default for RuleCtx {
    fn default() -> Self {
        RuleCtx {
            vault: "vault.test.near".to_string(),
            demand: Demande::Transfer {
                to: "bob.test.near".to_string(),
                amount: 0,
            },
            intent: Intent::default(),
            known: KnownContracts::new(),
            current_code: KnownContracts::new(),
            tx_cap: 2 * 10u128.pow(24),
            daily_cap: 10 * 10u128.pow(24),
            daily_spent: 0,
            fee_accounts: vec!["near".to_string(), "test.near".to_string()],
            gas_threshold: 100 * 10u64.pow(12),
            storage_threshold: 10 * 10u128.pow(24),
            max_touched: 8,
        }
    }
}

impl RuleCtx {
    fn is_fee_account(&self, account: &str) -> bool {
        self.fee_accounts.iter().any(|a| a == account)
    }

    fn is_beneficiary(&self, account: &str) -> bool {
        self.intent.beneficiaries.iter().any(|b| b == account)
    }

    /// Sortie de fonds du coffre pendant la simulation, en yoctoNEAR.
    pub fn outflow(&self, eff: &Effects) -> Yocto {
        eff.near_delta
            .iter()
            .filter(|d| d.account == self.vault && d.delta < 0)
            .map(|d| (-d.delta) as Yocto)
            .sum::<Yocto>()
    }
}

/// Juge les effets et rend un verdict.
pub fn judge(ctx: &RuleCtx, eff: &Effects) -> Decision {
    let mut hits: Vec<RuleHit> = Vec::new();
    for hit in [
        r1_caps(ctx, eff),
        r2_keys(ctx, eff),
        r3_approvals(ctx, eff),
        r4_transfer_call(ctx),
        r6_code_changed(ctx, eff),
        r7_undeclared(ctx, eff),
        r8_minimum(ctx, eff),
        r9_stranded(ctx, eff),
        r10_resources(ctx, eff),
        r11_touched(ctx, eff),
    ] {
        if let Some(h) = hit {
            hits.push(h);
        }
    }
    // R5 (premier contact) n'a de sens que si rien de plus grave n'a mordu.
    if !hits.iter().any(|h| h.id == "R4") {
        if let Some(h) = r5_first_contact(ctx) {
            hits.push(h);
        }
    }
    let verdict = verdict_of(&hits);
    Decision {
        verdict,
        rules: hits,
        effects: eff.clone(),
    }
}

/// Le verdict est la plus grave des règles qui ont mordu.
pub fn verdict_of(hits: &[RuleHit]) -> Verdict {
    let mut worst = Verdict::Allow;
    for h in hits {
        if h.severity.is_at_least(worst) {
            worst = h.severity;
        }
    }
    worst
}

/// R1 — sortie totale au-dessus du plafond par transaction ou du reste du jour.
fn r1_caps(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    let out = ctx.outflow(eff);
    if out == 0 {
        return None;
    }
    if out > ctx.tx_cap {
        return Some(RuleHit {
            id: "R1".into(),
            title: "sortie au-dessus du plafond".into(),
            detail: format!(
                "sortie de {} yoctoNEAR au-dessus du plafond par transaction ({})",
                out, ctx.tx_cap
            ),
            severity: Verdict::Block,
        });
    }
    let total = ctx.daily_spent.saturating_add(out);
    if total > ctx.daily_cap {
        return Some(RuleHit {
            id: "R1".into(),
            title: "sortie au-dessus du plafond du jour".into(),
            detail: format!(
                "sortie cumulée de {} yoctoNEAR au-dessus du plafond de 24 h ({})",
                total, ctx.daily_cap
            ),
            severity: Verdict::Block,
        });
    }
    None
}

/// R2 — une clé d'accès apparaît sur l'un de nos comptes.
fn r2_keys(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    eff.keys.iter().find_map(|k| match k {
        KeyEvent::Added { account, scope } if account == &ctx.vault => Some(RuleHit {
            id: "R2".into(),
            title: "clé d'accès ajoutée".into(),
            detail: format!("une clé d'accès ({}) a été ajoutée sur {}", scope, account),
            severity: Verdict::Block,
        }),
        _ => None,
    })
}

/// R3 — approbation de NFT ou de multi-jeton vers un compte hors liste.
fn r3_approvals(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    eff.nft.iter().find_map(|n| match n {
        NftEvent::Approve {
            collection,
            token,
            to,
        } => {
            if ctx.is_fee_account(to) || ctx.is_beneficiary(to) {
                return None;
            }
            Some(RuleHit {
                id: "R3".into(),
                title: "approbation hors liste".into(),
                detail: format!(
                    "approbation du jeton {} de la collection {} vers {}",
                    token, collection, to
                ),
                severity: Verdict::Block,
            })
        }
        _ => None,
    })
}

/// R4 — transfert avec appel (`ft_transfer_call`) vers un contrat inconnu.
fn r4_transfer_call(ctx: &RuleCtx) -> Option<RuleHit> {
    let (contract, method) = match &ctx.demand {
        Demande::FunctionCall {
            contract, method, ..
        } => (contract, method),
        Demande::Transfer { .. } => return None,
    };
    if !method.starts_with("ft_transfer_call") || ctx.known.contains_key(contract) {
        return None;
    }
    Some(RuleHit {
        id: "R4".into(),
        title: "transfert avec appel vers un inconnu".into(),
        detail: format!(
            "transfert avec appel ({}) vers le contrat inconnu {}",
            method, contract
        ),
        severity: Verdict::Block,
    })
}

/// R5 — premier contact avec un contrat.
fn r5_first_contact(ctx: &RuleCtx) -> Option<RuleHit> {
    let contract = ctx.demand.target()?;
    if ctx.known.contains_key(contract) {
        return None;
    }
    Some(RuleHit {
        id: "R5".into(),
        title: "premier contact avec un contrat".into(),
        detail: format!("premier contact avec le contrat {}", contract),
        severity: Verdict::Warn,
    })
}

/// R6 — contrat connu dont l'empreinte du code a changé.
fn r6_code_changed(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    for snap in &eff.snapshot.accounts {
        let account = &snap.id;
        let Some(agreed) = ctx.known.get(account) else {
            continue;
        };
        if snap.code_hash.is_empty() || &snap.code_hash == agreed {
            continue;
        }
        return Some(RuleHit {
            id: "R6".into(),
            title: "code du contrat changé".into(),
            detail: format!("le code de {} a changé depuis l'accord", account),
            severity: Verdict::Block,
        });
    }
    None
}

/// R7 — un bénéficiaire que l'intention ne nomme pas reçoit quelque chose.
fn r7_undeclared(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    let mut receivers: Vec<(String, String)> = Vec::new();
    for d in &eff.near_delta {
        if d.delta > 0 {
            receivers.push((d.account.clone(), format!("{} yoctoNEAR", d.delta)));
        }
    }
    for d in &eff.ft_delta {
        if d.delta > 0 {
            receivers.push((d.account.clone(), format!("{} jeton", d.delta)));
        }
    }
    for (account, what) in receivers {
        if account == ctx.vault || ctx.is_fee_account(&account) || ctx.is_beneficiary(&account) {
            continue;
        }
        return Some(RuleHit {
            id: "R7".into(),
            title: "bénéficiaire non déclaré".into(),
            detail: format!("{} a reçu {} sans être nommé dans l'intention", account, what),
            severity: Verdict::Block,
        });
    }
    None
}

/// R8 — reçu moins que le minimum déclaré, ou rien.
fn r8_minimum(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    for (token, min) in &ctx.intent.min_in {
        let got = eff.ft_delta_of(token, &ctx.vault);
        if (got as i128) < (*min as i128) {
            return Some(RuleHit {
                id: "R8".into(),
                title: "moins reçu que promis".into(),
                detail: format!(
                    "reçu {} de {} alors que l'intention demandait au moins {}",
                    got, token, min
                ),
                severity: Verdict::Block,
            });
        }
    }
    let out = ctx.outflow(eff);
    if out > 0 && ctx.intent.is_plain_transfer() {
        return Some(RuleHit {
            id: "R8".into(),
            title: "rien reçu".into(),
            detail: "les fonds sont sortis, rien n'a été reçu en échange".into(),
            severity: Verdict::Block,
        });
    }
    None
}

/// R9 — un reçu échoue après une sortie de fonds.
fn r9_stranded(_ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    if !eff.stranded {
        return None;
    }
    let failed: Vec<&str> = eff
        .receipts
        .iter()
        .filter(|r| r.status == "failed")
        .map(|r| r.receiver.as_str())
        .collect();
    Some(RuleHit {
        id: "R9".into(),
        title: "reçu en échec après la sortie".into(),
        detail: format!(
            "un reçu a échoué après la sortie de fonds (reçus en échec : {})",
            failed.join(", ")
        ),
        severity: Verdict::Block,
    })
}

/// R10 — gas brûlé ou dépôt de stockage au-dessus d'un seuil.
fn r10_resources(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    if eff.gas_burnt > ctx.gas_threshold {
        return Some(RuleHit {
            id: "R10".into(),
            title: "gas trop élevé".into(),
            detail: format!(
                "gas brûlé : {} au-dessus du seuil {}",
                eff.gas_burnt, ctx.gas_threshold
            ),
            severity: Verdict::Warn,
        });
    }
    if eff.storage_delta > ctx.storage_threshold {
        return Some(RuleHit {
            id: "R10".into(),
            title: "dépôt de stockage trop élevé".into(),
            detail: format!(
                "dépôt de stockage : {} au-dessus du seuil {}",
                eff.storage_delta, ctx.storage_threshold
            ),
            severity: Verdict::Warn,
        });
    }
    None
}

/// R11 — plus de huit comptes touchés.
fn r11_touched(ctx: &RuleCtx, eff: &Effects) -> Option<RuleHit> {
    let touched = eff.touched_accounts();
    if touched.len() > ctx.max_touched {
        return Some(RuleHit {
            id: "R11".into(),
            title: "trop de comptes touchés".into(),
            detail: format!(
                "{} comptes touchés, au maximum {}",
                touched.len(),
                ctx.max_touched
            ),
            severity: Verdict::Block,
        });
    }
    None
}
