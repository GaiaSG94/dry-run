//! Le bac à sable de simulation : on rejoue la transaction « en tant que le coffre ».
//!
//! Un bac à sable neuf est démarré pour **chaque** simulation (≈ 0,55 s) :
//! l'état repart toujours de la chaîne, aucune trace d'une simulation hostile
//! (clé posée, stockage créé) ne survit. Compromis documenté dans le README.

use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{Context, Result};
use near_crypto::SecretKey;
use near_sandbox::sandbox::patch::StateRecord;
use near_sandbox::Sandbox;
use near_token::NearToken;

use near_primitives::action::{Action, FunctionCallAction, TransferAction};
use near_primitives::transaction::SignedTransaction;
use near_primitives::types::{AccountId, Gas};

use dryrun_core::{Demande, Effects, Intent};

use crate::accounts;
use crate::chain::Chain;
use crate::effects;
use crate::outcome::{self, Execution};
use crate::rpc::{sign_transaction, Rpc};

/// Solde des comptes vides créés dans le bac à sable (assez pour payer le gas).
pub const EMPTY_BALANCE: u128 = 10 * 10u128.pow(24);

/// Clé plein accès posée sur le coffre dans le bac à sable de simulation.
pub const SIM_KEY_PUBLIC: &str = near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PUBLIC_KEY;
/// Clé privée correspondante (constante publique du bac à sable NEAR).
pub const SIM_KEY_PRIVATE: &str = near_sandbox::config::DEFAULT_GENESIS_ACCOUNT_PRIVATE_KEY;

/// Le bac à sable jetable qui rejoue les demandes.
pub struct SimSandbox {
    pub sandbox: Sandbox,
    pub rpc: Rpc,
}

impl SimSandbox {
    /// Démarre un bac à sable vierge.
    pub async fn start() -> Result<Self> {
        let sandbox = Sandbox::start_sandbox_with_config(crate::sandbox_cfg::config(Vec::new()))
            .await
            .context("démarrage du bac à sable de simulation")?;
        let rpc = Rpc::new(&sandbox.rpc_addr)?;
        Ok(SimSandbox { sandbox, rpc })
    }
}

/// Ce qu'une simulation a produit.
#[derive(Clone, Debug)]
pub struct Replay {
    pub effects: Effects,
    pub execution: Execution,
}

/// Rejoue une demande sur un bac à sable neuf et rend les effets observés.
pub async fn replay(chain: &Chain, demand: &Demande, intent: &Intent) -> Result<Replay> {
    let debut = std::time::Instant::now();
    let sim = SimSandbox::start().await?;
    let watched = watch_list(demand, intent);
    let token = match intent.min_in.first().map(|(t, _)| t.as_str()) {
        Some("TEST") => Some("test-token.test.near".to_string()),
        Some(other) if other.ends_with(".near") => Some(other.to_string()),
        _ => token_present(chain).await,
    };
    load_state(chain, &sim, demand, &watched, token.as_deref()).await?;
    let before = effects::snapshot(&sim.rpc, &watched, token.as_deref()).await?;
    let execution = run(&sim, demand).await?;
    let after = effects::snapshot(&sim.rpc, &watched, token.as_deref()).await?;
    tracing::debug!(resume = %demand.summary(), elapsed = ?debut.elapsed(), "rejeu terminé");
    let info = effects::ExecutionInfo {
        receipts: execution.receipts.clone(),
        gas_burnt: execution.gas_burnt,
        height: after.height,
        block_hash: after.block_hash.clone(),
    };
    // Si le réseau refuse la transaction avant exécution (solde insuffisant,
    // nonce, ...), on rapporte la sortie **demandée** : le pare-feu ne doit
    // jamais conclure ALLOW faute d'avoir pu observer quelque chose.
    let refused =
        (!execution.success && execution.receipts.is_empty()).then(|| effects::RejectedAttempt {
            amount: demand.amount(),
        });
    let produced = effects::effects(
        &before,
        &after,
        &info,
        accounts::VAULT,
        &[SIM_KEY_PUBLIC.to_string()],
        refused.as_ref(),
    );
    Ok(Replay {
        effects: produced,
        execution,
    })
}

/// Comptes à observer : le coffre, la cible, les bénéficiaires, les comptes de la démo.
pub fn watch_list(demand: &Demande, intent: &Intent) -> Vec<String> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    let mut push = |name: &str, out: &mut Vec<String>| {
        if seen.insert(name.to_string()) {
            out.push(name.to_string());
        }
    };
    push(accounts::VAULT, &mut out);
    if let Some(target) = demand.target() {
        push(target, &mut out);
    }
    if let Demande::Transfer { to, .. } = demand {
        push(to, &mut out);
    }
    for name in intent.beneficiaries.iter().map(String::as_str) {
        push(name, &mut out);
    }
    for name in accounts::WATCHED_PLAIN {
        push(name, &mut out);
    }
    // Les registres de jetons sont toujours copiés : un piège peut toucher un
    // jeton ou un NFT sans que l'intention le mentionne (règles R3 et R7).
    push(accounts::TEST_TOKEN, &mut out);
    push(accounts::TEST_NFT, &mut out);
    out
}

/// Le jeton de démonstration est-il présent sur la chaîne ?
async fn token_present(chain: &Chain) -> Option<String> {
    let token = effects::watched_token();
    match chain.rpc.account(&token.parse().ok()?).await {
        Ok(Some(info)) if info.has_code() => Some(token.to_string()),
        _ => None,
    }
}

/// Recharge dans le bac à sable de simulation l'état utile de la chaîne.
///
/// Compromis assumé : le coffre, la cible et le jeton sont copiés
/// intégralement (compte + stockage + code + clés) ; les autres comptes
/// seulement pour leur compte et leurs clés, ce qui suffit à lire les deltas
/// et divise le volume transféré.
///
/// Les enregistrements partent en **un seul** `patch_state` : chaque envoi
/// coûte une attente de bloc (≈ 0,4 s), c'est le principal poste de latence.
async fn load_state(
    chain: &Chain,
    sim: &SimSandbox,
    demand: &Demande,
    watched: &[String],
    token: Option<&str>,
) -> Result<()> {
    let mut records = Vec::new();
    let mut vault_seen = false;
    for name in watched {
        let Ok(account) = AccountId::from_str(name) else {
            continue;
        };
        // Tous les contrats de la démo sont copiés intégralement : un appel
        // inter-contrats peut en toucher plusieurs (DEX → jeton → coffre).
        let full = name == accounts::VAULT
            || Some(name.as_str()) == demand.target()
            || Some(name.as_str()) == token
            || accounts::INSTRUMENTS.contains(&name.as_str())
            || accounts::CONTRACTS
                .iter()
                .any(|spec| spec.account == name.as_str());
        let recs = records_of(chain, &account, full).await?;
        if name == accounts::TEST_NFT || name == accounts::TEST_TOKEN || name == accounts::VAULT {
            tracing::info!(%name, full, records = recs.len(), "état copié");
        }
        records.extend(recs);
        vault_seen |= name == accounts::VAULT;
    }
    // Comptes vides que la transaction peut toucher (ex. `ghost.test.near`
    // appelé par le piège `half-step`) : le bac à sable doit les connaître,
    // sinon le reçu échoue pour une mauvaise raison (compte inexistant).
    for name in accounts::EMPTY_ACCOUNTS.iter() {
        let Ok(account) = AccountId::from_str(name) else {
            continue;
        };
        records.push(StateRecord::Account {
            account_id: account,
            account: serde_json::json!({
                "amount": EMPTY_BALANCE.to_string(),
                "locked": "0",
                "code_hash": "11111111111111111111111111111111",
                "storage_usage": 0,
            }),
        });
    }
    if !vault_seen {
        anyhow::bail!("le compte du coffre n'existe pas sur la chaîne : déployez d'abord");
    }
    let vault: AccountId = accounts::VAULT.parse()?;
    let mut patch = sim.sandbox.patch_state(vault);
    for record in records {
        patch = patch.state_record(record);
    }
    patch
        .with_default_access_key()
        .send()
        .await
        .context("copie de l'état vers le bac à sable de simulation")?;
    Ok(())
}

/// Enregistrements d'état d'un compte, prêts pour `sandbox_patch_state`.
async fn records_of(chain: &Chain, account: &AccountId, full: bool) -> Result<Vec<StateRecord>> {
    let mut records = Vec::new();
    let Some(view) = chain.rpc.account_json(account).await? else {
        tracing::debug!(%account, "compte absent de la chaîne");
        return Ok(records);
    };
    records.push(StateRecord::Account {
        account_id: account.clone(),
        account: view,
    });
    for (public_key, access_key) in chain.rpc.access_keys_json(account).await? {
        records.push(StateRecord::AccessKey {
            account_id: account.clone(),
            public_key_base64: public_key,
            access_key,
        });
    }
    if !full {
        return Ok(records);
    }
    if let Some(code_base64) = chain.rpc.code_b64(account).await? {
        records.push(StateRecord::Contract {
            account_id: account.clone(),
            code_base64,
        });
    }
    for (data_key_base64, value_base64) in chain.rpc.state_items(account).await? {
        records.push(StateRecord::Data {
            account_id: account.clone(),
            data_key_base64,
            value_base64,
        });
    }
    Ok(records)
}

/// Joue la demande depuis le compte du coffre.
async fn run(sim: &SimSandbox, demand: &Demande) -> Result<Execution> {
    let vault: AccountId = accounts::VAULT.parse()?;
    let key = SecretKey::from_str(SIM_KEY_PRIVATE).context("clé de simulation invalide")?;
    let nonce = sim
        .rpc
        .key_nonce(&vault, &key.public_key())
        .await?
        .unwrap_or(0)
        + 1;
    let block_hash = sim.rpc.block_hash().await?;
    let (receiver, action) = action_of(demand)?;
    let tx: SignedTransaction =
        sign_transaction(&key, &vault, &receiver, nonce, block_hash, vec![action]);
    match sim.rpc.send(&tx).await {
        Ok(outcome) => Ok(outcome::parse(&outcome)),
        Err(error) => Ok(Execution {
            success: false,
            error: Some(format!("{error:#}")),
            ..Execution::default()
        }),
    }
}

/// Traduit une demande en action NEAR et en destinataire.
fn action_of(demand: &Demande) -> Result<(AccountId, Action)> {
    match demand {
        Demande::Transfer { to, amount } => {
            let receiver: AccountId = to
                .parse()
                .with_context(|| format!("destinataire invalide : {to}"))?;
            let action = Action::Transfer(TransferAction {
                deposit: NearToken::from_yoctonear(*amount),
            });
            Ok((receiver, action))
        }
        Demande::FunctionCall {
            contract,
            method,
            args,
            deposit,
            gas,
        } => {
            let receiver: AccountId = contract
                .parse()
                .with_context(|| format!("contrat invalide : {contract}"))?;
            // Les arguments doivent être un objet JSON dont les clés correspondent
            // aux paramètres de la méthode (near-sdk 5). Une chaîne seule ou `Null`
            // est enveloppée en `{"args": "..."}` pour les pièges `trap(args)`.
            let encoded = match args {
                serde_json::Value::Null => serde_json::to_vec(&serde_json::json!({"args": ""}))?,
                serde_json::Value::String(text) => {
                    serde_json::to_vec(&serde_json::json!({"args": text}))?
                }
                other => serde_json::to_vec(other).context("encodage des arguments")?,
            };
            let action = Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: method.clone(),
                args: encoded,
                gas: Gas::from_gas(*gas),
                deposit: NearToken::from_yoctonear(*deposit),
            }));
            Ok((receiver, action))
        }
    }
}
