//! Le bac à sable de simulation : on rejoue la transaction « en tant que le coffre ».
//!
//! Un bac à sable neuf est démarré pour **chaque** simulation : l'état repart
//! toujours de la chaîne, aucune trace d'une simulation hostile ne survit.
//! Compromis assumé (≈ 1 s de démarrage) documenté dans le README du crate.

use std::collections::BTreeSet;
use std::str::FromStr;

use anyhow::{Context, Result};
use near_crypto::SecretKey;
use near_sandbox::{FetchData, Sandbox};
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
        let sandbox = Sandbox::start_sandbox()
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
    let sim = SimSandbox::start().await?;
    let watched = watch_list(demand, intent);
    let token = token_present(chain).await;
    load_state(chain, &sim, demand, &watched, token.as_deref()).await?;
    let before = effects::snapshot(&sim.rpc, &watched, token.as_deref()).await?;
    let execution = run(&sim, demand).await?;
    let after = effects::snapshot(&sim.rpc, &watched, token.as_deref()).await?;
    let info = effects::ExecutionInfo {
        receipts: execution.receipts.clone(),
        gas_burnt: execution.gas_burnt,
        height: after.height,
        block_hash: after.block_hash.clone(),
    };
    let produced = effects::effects(
        &before,
        &after,
        &info,
        accounts::VAULT,
        &[SIM_KEY_PUBLIC.to_string()],
    );
    Ok(Replay { effects: produced, execution })
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
/// Compromis assumé : seuls le coffre, la cible et le jeton sont copiés
/// intégralement (code + stockage + clés). Les autres comptes ne sont copiés
/// que pour leur solde et leurs clés, ce qui suffit à lire les deltas et
/// divise le volume de données transférées.
async fn load_state(
    chain: &Chain,
    sim: &SimSandbox,
    demand: &Demande,
    watched: &[String],
    token: Option<&str>,
) -> Result<()> {
    let mut vault_copied = false;
    for name in watched {
        let full = name == accounts::VAULT
            || Some(name.as_str()) == demand.target()
            || Some(name.as_str()) == token;
        let data = if full { FetchData::ALL } else { FetchData::new().account().access_keys() };
        let is_vault = name == accounts::VAULT;
        if copy(chain, sim, name, data, is_vault).await? && is_vault {
            vault_copied = true;
        }
    }
    if !vault_copied {
        anyhow::bail!("le compte du coffre n'existe pas sur la chaîne : déployez d'abord");
    }
    Ok(())
}

/// Copie un compte de la chaîne vers le bac à sable ; rend `false` s'il n'existe pas.
async fn copy(
    chain: &Chain,
    sim: &SimSandbox,
    name: &str,
    data: FetchData,
    signable: bool,
) -> Result<bool> {
    let Ok(account) = AccountId::from_str(name) else { return Ok(false) };
    let mut patch = match sim.sandbox.patch_state(account).fetch_from(chain.rpc_url(), data).await {
        Ok(patch) => patch,
        Err(error) => {
            tracing::debug!(account = name, %error, "compte absent de la chaîne");
            return Ok(false);
        }
    };
    if signable {
        patch = patch.with_default_access_key();
    }
    match patch.send().await {
        Ok(()) => Ok(true),
        Err(error) => {
            tracing::debug!(account = name, %error, "compte non copié");
            Ok(false)
        }
    }
}

/// Joue la demande depuis le compte du coffre.
async fn run(sim: &SimSandbox, demand: &Demande) -> Result<Execution> {
    let vault: AccountId = accounts::VAULT.parse()?;
    let key = SecretKey::from_str(SIM_KEY_PRIVATE).context("clé de simulation invalide")?;
    let nonce = sim.rpc.key_nonce(&vault, &key.public_key()).await?.unwrap_or(0) + 1;
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
            let receiver: AccountId = to.parse().with_context(|| format!("destinataire invalide : {to}"))?;
            let action = Action::Transfer(TransferAction {
                deposit: NearToken::from_yoctonear(*amount),
            });
            Ok((receiver, action))
        }
        Demande::FunctionCall { contract, method, args, deposit, gas } => {
            let receiver: AccountId =
                contract.parse().with_context(|| format!("contrat invalide : {contract}"))?;
            let encoded = serde_json::to_vec(args).context("encodage des arguments")?;
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

