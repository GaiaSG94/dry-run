//! La chaîne locale : un bac à sable `near-sandbox` qui porte contrats et comptes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use near_crypto::{KeyType, SecretKey};
use near_sandbox::{GenesisAccount, Sandbox};
use near_token::NearToken;

use near_primitives::account::{AccessKey, AccessKeyPermission, FunctionCallPermission};
use near_primitives::action::{Action, AddKeyAction, DeleteKeyAction, DeployContractAction};
use near_primitives::types::{AccountId, Gas};
use serde_json::{json, Value};

use crate::accounts::{self, ContractSpec};
use crate::outcome::{self, Execution};
use crate::rpc::{sign_transaction, AccountInfo, Rpc};
use crate::util::{amount_of, scope_of};

/// Résultat d'un appel : alias de [`Execution`] pour coller au vocabulaire de la SPEC.
pub type TxOutcome = Execution;

/// Gaz alloué aux appels d'initialisation.
const INIT_GAS: u64 = 100_000_000_000_000;
/// Allocation de la clé d'agent (1 NEAR), en yoctoNEAR.
pub const AGENT_ALLOWANCE: u128 = 10u128.pow(24);
/// Allocation de la clé de gel, en yoctoNEAR — **0**, comme le veut la SPEC.
///
/// Conséquence à connaître : sur NEAR, le gas d'une transaction est prélevé sur
/// l'allocation ; une allocation nulle rend la clé inutilisable
/// (`NotEnoughAllowance`). La clé est donc bien installée avec 0, et
/// [`Chain::freeze`] se replie sur le propriétaire en le signalant.
pub const FREEZE_ALLOWANCE: u128 = 0;
/// Solde donné à chaque compte de la démo à la genèse (en NEAR).
const FUNDS: u128 = 100;
/// Solde du coffre à la genèse (en NEAR).
const VAULT_FUNDS: u128 = 50;

/// État d'un compte lu sur la chaîne.
#[derive(Clone, Debug, Default)]
pub struct AccountState {
    pub account: String,
    pub balance: u128,
    pub code_hash: String,
    pub storage_usage: u64,
    pub keys: Vec<String>,
    /// Soldes des jetons fongibles connus : `(jeton, montant)`.
    pub ft: Vec<(String, u128)>,
}

/// Résultat du déploiement.
#[derive(Clone, Debug, Default)]
pub struct Deployed {
    /// Compte → empreinte du code déployé.
    pub contracts: BTreeMap<String, String>,
    /// Clés installées sur le coffre : `(clé publique, portée)`.
    pub vault_keys: Vec<(String, String)>,
    /// Le coffre a-t-il bien zéro clé d'accès total ?
    pub vault_locked: bool,
}

/// La chaîne locale de la démonstration.
pub struct Chain {
    pub sandbox: Sandbox,
    pub rpc: Rpc,
    wasm_dir: PathBuf,
    keys: BTreeMap<String, SecretKey>,
    /// Clé limitée à `execute`, installée sur le coffre au déploiement.
    agent_key: SecretKey,
    /// Clé limitée à `freeze`.
    freeze_key: SecretKey,
}

impl Chain {
    /// Démarre la chaîne avec les comptes de génèse de la démo.
    pub async fn start(wasm_dir: &Path) -> Result<Self> {
        let mut keys = BTreeMap::new();
        let mut genesis = Vec::new();
        for name in accounts::GENESIS_ACCOUNTS {
            let secret = SecretKey::from_random(KeyType::ED25519);
            let funds = match *name {
                accounts::VAULT => VAULT_FUNDS,
                _ => FUNDS,
            };
            genesis.push(GenesisAccount {
                account_id: name
                    .parse()
                    .with_context(|| format!("compte invalide : {name}"))?,
                public_key: secret.public_key().to_string(),
                private_key: secret.to_string(),
                balance: NearToken::from_near(funds),
            });
            keys.insert((*name).to_string(), secret);
        }
        let sandbox = Sandbox::start_sandbox_with_config(crate::sandbox_cfg::config(genesis))
            .await
            .context("démarrage du bac à sable de la chaîne")?;
        let rpc = Rpc::new(&sandbox.rpc_addr)?;
        Ok(Chain {
            sandbox,
            rpc,
            wasm_dir: wasm_dir.to_path_buf(),
            keys,
            agent_key: SecretKey::from_random(KeyType::ED25519),
            freeze_key: SecretKey::from_random(KeyType::ED25519),
        })
    }

    /// Adresse RPC de la chaîne.
    pub fn rpc_url(&self) -> &str {
        self.rpc.url()
    }

    /// Hauteur du dernier bloc.
    pub async fn height(&self) -> Result<u64> {
        self.rpc.height().await
    }

    /// Clé limitée à `execute` (celle que détiendrait le garde-barrière).
    pub fn agent_key(&self) -> &SecretKey {
        &self.agent_key
    }

    /// Clé limitée à `freeze` (celle du gardien).
    pub fn freeze_key(&self) -> &SecretKey {
        &self.freeze_key
    }

    /// Clé de signature d'un compte (comptes de génèse).
    pub fn key_of(&self, account: &str) -> Result<&SecretKey> {
        self.keys
            .get(account)
            .ok_or_else(|| anyhow!("aucune clé connue pour {account}"))
    }

    /// Déploie tous les contrats de démonstration dont le wasm est présent.
    ///
    /// Les fichiers absents sont ignorés : le crate reste utilisable pendant
    /// que les contrats des autres agents se compilent.
    pub async fn deploy_all(&self) -> Result<Deployed> {
        let mut deployed = Deployed::default();
        for spec in accounts::CONTRACTS {
            match self.deploy_one(*spec).await? {
                Some(code_hash) => {
                    deployed
                        .contracts
                        .insert(spec.account.to_string(), code_hash);
                }
                None => tracing::warn!(wasm = spec.wasm, "wasm absent : contrat non déployé"),
            }
        }
        if deployed.contracts.contains_key(accounts::VAULT) {
            let vault: AccountId = accounts::VAULT.parse()?;
            deployed.vault_keys = self
                .rpc
                .access_keys(&vault)
                .await?
                .into_iter()
                .map(|key| {
                    (
                        key.public_key.to_string(),
                        scope_of(&key.access_key.permission),
                    )
                })
                .collect();
            deployed.vault_locked = self.vault_has_no_full_access().await?;
        }
        Ok(deployed)
    }

    /// Déploie un contrat, verrouille le coffre si c'en est un.
    ///
    /// Idempotent : si le compte porte déjà exactement ce wasm, rien n'est
    /// renvoyé (la clé plein accès du coffre n'existe plus après le premier
    /// déploiement, un second essai échouerait).
    async fn deploy_one(&self, spec: ContractSpec) -> Result<Option<String>> {
        let Some(path) = accounts::wasm_path(&self.wasm_dir, spec.wasm) else {
            return Ok(None);
        };
        let code =
            std::fs::read(&path).with_context(|| format!("lecture de {}", path.display()))?;
        let account: AccountId = spec.account.parse()?;
        if self.already_deployed(&account, &code).await? {
            tracing::debug!(account = spec.account, "contrat déjà déployé");
            let hash = self
                .rpc
                .account(&account)
                .await?
                .map(|info| info.code_hash)
                .unwrap_or_default();
            return Ok(Some(hash));
        }
        let signer = self.key_of(spec.account)?.clone();
        let mut actions = vec![Action::DeployContract(DeployContractAction { code })];
        if spec.is_vault {
            let init_args = serde_json::to_vec(&serde_json::json!({
                "owner": accounts::OWNER,
                "tx_cap": (2 * 10u128.pow(24)).to_string(),
                "daily_cap": (10 * 10u128.pow(24)).to_string(),
            }))?;
            actions.push(Action::FunctionCall(Box::new(
                near_primitives::transaction::FunctionCallAction {
                    method_name: "new".to_string(),
                    args: init_args,
                    gas: near_primitives::types::Gas::from_teragas(30),
                    deposit: near_token::NearToken::from_yoctonear(0),
                },
            )));
            actions.extend(lock_actions(&signer, &self.agent_key, &self.freeze_key));
        }
        let outcome = self
            .send(&signer, &account, actions)
            .await
            .with_context(|| format!("déploiement de {}", spec.account))?;
        if !outcome.success {
            bail!(
                "déploiement de {} refusé : {}",
                spec.account,
                outcome.error.clone().unwrap_or_default()
            );
        }
        let code_hash = self
            .rpc
            .account(&account)
            .await?
            .map(|info| info.code_hash)
            .ok_or_else(|| anyhow!("compte {} absent après déploiement", spec.account))?;
        if spec.is_vault {
            self.init_vault().await?;
        }
        Ok(Some(code_hash))
    }

    /// Le compte porte-t-il déjà exactement ces octets de code ?
    async fn already_deployed(&self, account: &AccountId, code: &[u8]) -> Result<bool> {
        let deployed = self.rpc.code_b64(account).await?;
        Ok(deployed.as_deref() == Some(crate::rpc::b64(code).as_str()))
    }

    /// Initialise le coffre : propriétaire, plafonds (2 NEAR / 10 NEAR).
    async fn init_vault(&self) -> Result<()> {
        let args = json!({
            "owner": accounts::OWNER,
            "tx_cap": (2 * 10u128.pow(24)).to_string(),
            "daily_cap": (10 * 10u128.pow(24)).to_string(),
        });
        match self
            .call(accounts::OWNER, accounts::VAULT, "new", &args, 0, INIT_GAS)
            .await
        {
            Ok(outcome) if outcome.success => Ok(()),
            other => {
                tracing::warn!(?other, "initialisation du coffre ignorée");
                Ok(())
            }
        }
    }

    /// Le compte du coffre n'a-t-il plus aucune clé d'accès total ?
    pub async fn vault_has_no_full_access(&self) -> Result<bool> {
        let account: AccountId = accounts::VAULT.parse()?;
        let keys = self.rpc.access_keys(&account).await?;
        Ok(keys
            .iter()
            .all(|key| scope_of(&key.access_key.permission) != "full_access"))
    }

    /// Appel `view` sur la chaîne.
    pub async fn view_call(&self, account: &str, method: &str, args: &Value) -> Result<Value> {
        let account: AccountId = account.parse()?;
        self.rpc.view_call(&account, method, args).await
    }

    /// Envoie un appel de fonction signé par un compte de génèse.
    pub async fn call(
        &self,
        signer: &str,
        receiver: &str,
        method: &str,
        args: &Value,
        deposit: u128,
        gas: u64,
    ) -> Result<TxOutcome> {
        let key = self.key_of(signer)?.clone();
        let receiver_id: AccountId = receiver.parse()?;
        let encoded = serde_json::to_vec(args).context("encodage des arguments")?;
        let action = Action::FunctionCall(Box::new(near_primitives::action::FunctionCallAction {
            method_name: method.to_string(),
            args: encoded,
            gas: Gas::from_gas(gas),
            deposit: NearToken::from_yoctonear(deposit),
        }));
        self.send(&key, &receiver_id, vec![action]).await
    }

    /// Appelle le coffre avec la clé limitée à `execute` (ou `freeze`).
    ///
    /// C'est le chemin de l'exécution réelle : la clé d'agent ne peut appeler
    /// que la méthode autorisée, et rien d'autre sur le réseau.
    pub async fn call_with_agent(
        &self,
        receiver: &str,
        method: &str,
        args: &Value,
        gas: u64,
    ) -> Result<TxOutcome> {
        let key = self.agent_key.clone();
        let receiver_id: AccountId = receiver.parse()?;
        let encoded = serde_json::to_vec(args).context("encodage des arguments")?;
        let action = Action::FunctionCall(Box::new(near_primitives::action::FunctionCallAction {
            method_name: method.to_string(),
            args: encoded,
            gas: Gas::from_gas(gas),
            deposit: NearToken::from_yoctonear(0),
        }));
        let vault: AccountId = accounts::VAULT.parse()?;
        let nonce = self
            .rpc
            .key_nonce(&vault, &key.public_key())
            .await?
            .unwrap_or(0)
            + 1;
        let block_hash = self.rpc.block_hash().await?;
        let tx = sign_transaction(&key, &vault, &receiver_id, nonce, block_hash, vec![action]);
        let outcome = self.rpc.send(&tx).await?;
        Ok(outcome::parse(&outcome))
    }

    /// Gèle le coffre : d'abord avec la clé de gel, puis avec le propriétaire.
    ///
    /// La clé de gel a une allocation de 0 (SPEC) : NEAR refuse alors de payer
    /// le gas, on se rabat donc sur le propriétaire — qui a de toute façon le
    /// droit de geler. Le repli est signalé dans les journaux du résultat.
    pub async fn freeze(&self, receiver: &str) -> Result<TxOutcome> {
        let guardian = self.freeze_key.clone();
        match self.send_freeze_with(&guardian, receiver).await {
            Ok(outcome) if outcome.success => Ok(outcome),
            Ok(outcome) => Ok(self
                .send_freeze_with(self.key_of(accounts::OWNER)?, receiver)
                .await
                .map(|mut fallback| {
                    fallback.logs.push(format!(
                        "clé de gel sans allocation, gel fait par le propriétaire ({:?})",
                        outcome.error
                    ));
                    fallback
                })?),
            Err(error) => {
                let key = self.key_of(accounts::OWNER)?.clone();
                let mut fallback = self
                    .send_freeze_with(&key, receiver)
                    .await
                    .map_err(|_| error)?;
                fallback
                    .logs
                    .push("clé de gel sans allocation, gel fait par le propriétaire".to_string());
                Ok(fallback)
            }
        }
    }

    /// Envoie l'appel `freeze` signé par une clé donnée.
    async fn send_freeze_with(&self, key: &SecretKey, receiver: &str) -> Result<TxOutcome> {
        let receiver_id: AccountId = receiver.parse()?;
        let action = Action::FunctionCall(Box::new(near_primitives::action::FunctionCallAction {
            method_name: "freeze".to_string(),
            args: b"{}".to_vec(),
            gas: Gas::from_gas(INIT_GAS),
            deposit: NearToken::from_yoctonear(0),
        }));
        let signer: AccountId = self.account_of_key(key)?.parse()?;
        let nonce = self
            .rpc
            .key_nonce(&signer, &key.public_key())
            .await?
            .unwrap_or(0)
            + 1;
        let block_hash = self.rpc.block_hash().await?;
        let tx = sign_transaction(key, &signer, &receiver_id, nonce, block_hash, vec![action]);
        let outcome = self.rpc.send(&tx).await?;
        Ok(outcome::parse(&outcome))
    }

    /// Envoie des actions signées par `signer` vers `receiver`.
    async fn send(
        &self,
        signer: &SecretKey,
        receiver: &AccountId,
        actions: Vec<Action>,
    ) -> Result<TxOutcome> {
        let signer_id: AccountId = self.account_of_key(signer)?.parse()?;
        let nonce = self
            .rpc
            .key_nonce(&signer_id, &signer.public_key())
            .await?
            .unwrap_or(0)
            + 1;
        let block_hash = self.rpc.block_hash().await?;
        let tx = sign_transaction(signer, &signer_id, receiver, nonce, block_hash, actions);
        Ok(send_parsed(&self.rpc, &tx).await)
    }

    /// Retrouve le nom du compte porteur d'une clé.
    fn account_of_key(&self, key: &SecretKey) -> Result<String> {
        let public = key.public_key().to_string();
        self.keys
            .iter()
            .find(|(_, secret)| secret.public_key().to_string() == public)
            .map(|(name, _)| name.clone())
            .ok_or_else(|| anyhow!("clé inconnue : {public}"))
    }

    /// État complet d'un compte (code, clés, solde NEAR, soldes FT).
    pub async fn state_of(&self, account: &str) -> Result<AccountState> {
        let account_id: AccountId = account.parse()?;
        let info: Option<AccountInfo> = self.rpc.account(&account_id).await?;
        let mut state = AccountState {
            account: account.to_string(),
            balance: info.as_ref().map(|i| i.amount.as_yoctonear()).unwrap_or(0),
            code_hash: info
                .as_ref()
                .map(|i| i.code_hash.clone())
                .unwrap_or_default(),
            storage_usage: info.as_ref().map(|i| i.storage_usage).unwrap_or(0),
            keys: self
                .rpc
                .access_keys(&account_id)
                .await?
                .iter()
                .map(|key| key.public_key.to_string())
                .collect(),
            ft: Vec::new(),
        };
        state.ft = self.ft_balances(account).await?;
        Ok(state)
    }

    /// Soldes de jeton fongible d'un compte sur le jeton de démonstration.
    pub async fn ft_balances(&self, account: &str) -> Result<Vec<(String, u128)>> {
        let token = accounts::TEST_TOKEN;
        let token_id: AccountId = token.parse()?;
        let Some(info) = self.rpc.account(&token_id).await? else {
            return Ok(Vec::new());
        };
        if !info.has_code() {
            return Ok(Vec::new());
        }
        let args = json!({"account_id": account});
        let value = match self.rpc.view_call(&token_id, "ft_balance_of", &args).await {
            Ok(value) => value,
            Err(_) => return Ok(Vec::new()),
        };
        Ok(vec![(token.to_string(), amount_of(&value).unwrap_or(0))])
    }
}

/// Actions qui verrouillent le coffre : clé d'agent, clé de gel, retrait du plein accès.
fn lock_actions(signer: &SecretKey, agent: &SecretKey, freeze: &SecretKey) -> Vec<Action> {
    vec![
        Action::AddKey(Box::new(AddKeyAction {
            public_key: agent.public_key(),
            access_key: function_call_key(AGENT_ALLOWANCE, "execute"),
        })),
        Action::AddKey(Box::new(AddKeyAction {
            public_key: freeze.public_key(),
            access_key: function_call_key(FREEZE_ALLOWANCE, "freeze"),
        })),
        Action::DeleteKey(Box::new(DeleteKeyAction {
            public_key: signer.public_key(),
        })),
    ]
}

/// Envoie une transaction ; un refus du réseau devient une exécution en échec
/// (et non une erreur de transport), pour que l'appelant garde la main.
async fn send_parsed(rpc: &Rpc, tx: &near_primitives::transaction::SignedTransaction) -> Execution {
    match rpc.send(tx).await {
        Ok(outcome) => outcome::parse(&outcome),
        Err(error) => Execution {
            success: false,
            error: Some(format!("{error:#}")),
            ..Execution::default()
        },
    }
}

/// Une clé limitée à une seule méthode du coffre.
fn function_call_key(allowance: u128, method: &str) -> AccessKey {
    AccessKey {
        nonce: 0,
        permission: AccessKeyPermission::FunctionCall(FunctionCallPermission {
            allowance: Some(NearToken::from_yoctonear(allowance)),
            receiver_id: accounts::VAULT.to_string(),
            method_names: vec![method.to_string()],
        }),
    }
}
