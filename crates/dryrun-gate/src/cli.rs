//! Analyse des arguments de la ligne de commande, à la main (sans `clap`).

use dryrun_core::types::{Demande, Intent, Yocto};

use crate::lang::Lang;

/// Réseau sur lequel le garde-barrière travaille.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Network {
    /// Coffre en mémoire ou bac à sable local (défaut, comportement inchangé).
    Local,
    /// Coffre réellement déployé sur une chaîne NEAR publique.
    Chain {
        /// Compte du coffre (il porte aussi le contrat `vault`).
        vault: String,
        /// Nom du réseau pour la CLI NEAR (`testnet`).
        name: String,
        /// URL RPC du réseau.
        rpc_url: String,
    },
}

impl Network {
    /// Réseau local (par défaut).
    pub fn local() -> Self {
        Network::Local
    }

    /// Réseau testnet public. `DRYRUN_RPC_URL` remplace le point d'accès RPC par défaut
    /// (rpc.testnet.near.org est déprécié et limite le débit : erreurs 429).
    pub fn testnet(vault: String) -> Self {
        let rpc_url = std::env::var("DRYRUN_RPC_URL")
            .ok()
            .filter(|u| u.starts_with("https://"))
            .unwrap_or_else(|| crate::testnet::rpc::TESTNET_RPC_URL.to_string());
        Network::Chain {
            vault,
            name: "testnet".to_string(),
            rpc_url,
        }
    }
}

/// Commande demandée à la CLI.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// Propose une demande au pare-feu.
    Propose { demand: Demande, intent: Intent },
    /// Approuve un contrat (action du propriétaire).
    Approve { contract: String },
    /// Gèle le coffre d'un geste.
    Freeze,
    /// Affiche l'état du coffre.
    Status,
    /// Affiche la frise des demandes.
    Log,
    /// Déroule la démonstration guidée du garde-barrière.
    Demo,
    /// Joue le corpus d'attaques et vérifie que tout est refusé.
    Attack,
    /// Sert la page web et l'API JSON.
    Serve {
        port: u16,
        web_dir: String,
        network: Network,
    },
    /// Lance le banc de test automatisé de 200 robots (matrice de confusion).
    Sim { seed: u64 },
    /// Démarre le serveur MCP JSON-RPC sur stdin/stdout pour les agents IA.
    Mcp { network: Network, lang: Lang },
    /// Affiche l'aide.
    Help,
}

/// Convertit un montant décimal de jeton avec un nombre donné de décimales.
pub fn parse_token_amount(text: &str, decimals: u32) -> anyhow::Result<Yocto> {
    let trimmed = text.trim();
    let mut parts = trimmed.split('.');
    let whole_text = parts.next().unwrap_or("0");
    let frac_text = parts.next().unwrap_or("");
    if parts.next().is_some() {
        anyhow::bail!("montant invalide : {trimmed}");
    }
    let whole: u128 = if whole_text.is_empty() {
        0
    } else {
        whole_text
            .parse()
            .map_err(|_| anyhow::anyhow!("montant invalide : {trimmed}"))?
    };
    let mut frac = frac_text.to_string();
    frac.truncate(decimals as usize);
    while frac.len() < decimals as usize {
        frac.push('0');
    }
    let frac: u128 = if frac.is_empty() {
        0
    } else {
        frac.parse()
            .map_err(|_| anyhow::anyhow!("montant invalide : {trimmed}"))?
    };
    Ok(whole * 10u128.pow(decimals) + frac)
}

/// Convertit un montant NEAR décimal en yoctoNEAR (1 NEAR = 10^24 yocto).
pub fn parse_near(text: &str) -> anyhow::Result<Yocto> {
    parse_token_amount(text, 24)
}

/// Options brutes de la sous-commande `propose`.
#[derive(Clone, Default)]
struct ProposeOptions {
    to: Option<String>,
    amount: Option<String>,
    call: Option<String>,
    method: Option<String>,
    args: Option<String>,
    deposit: Option<String>,
    intent: Option<String>,
    beneficiaries: Vec<String>,
    min_in: Vec<String>,
}

impl ProposeOptions {
    /// Lit les options une à une, en consommant leur valeur.
    fn parse(args: &[String]) -> anyhow::Result<Self> {
        let mut opts = ProposeOptions::default();
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            let value = match args.get(index + 1) {
                Some(v) => v.clone(),
                None => anyhow::bail!("valeur manquante après {flag}"),
            };
            match flag {
                "--to" => opts.to = Some(value),
                "--amount" => opts.amount = Some(value),
                "--call" => opts.call = Some(value),
                "--method" => opts.method = Some(value),
                "--args" => opts.args = Some(value),
                "--deposit" => opts.deposit = Some(value),
                "--intent" => opts.intent = Some(value),
                "--beneficiary" => opts.beneficiaries.push(value),
                "--min-in" => opts.min_in.push(value),
                other => anyhow::bail!("option inconnue : {other}"),
            }
            index += 2;
        }
        Ok(opts)
    }

    /// Construit la demande proposée au pare-feu.
    fn into_demand(self) -> anyhow::Result<Demande> {
        if let (Some(contract), Some(method)) = (self.call.clone(), self.method.clone()) {
            let args = match self.args.as_deref() {
                Some(raw) => serde_json::from_str(raw)
                    .map_err(|err| anyhow::anyhow!("--args n'est pas du JSON valide : {err}"))?,
                None => serde_json::Value::Null,
            };
            let deposit = match self.deposit.as_deref() {
                Some(raw) => parse_near(raw)?,
                None => 0,
            };
            return Ok(Demande::FunctionCall {
                contract,
                method,
                args,
                deposit,
                gas: 30_000_000_000_000,
            });
        }

        match (self.to, self.amount) {
            (Some(to), Some(amount)) => Ok(Demande::Transfer {
                to,
                amount: parse_near(&amount)?,
            }),
            _ => anyhow::bail!(
                "demande incomplète : fournir --to et --amount, ou --call et --method"
            ),
        }
    }

    /// Construit l'intention déclarée (obligatoire).
    fn into_intent(self) -> anyhow::Result<Intent> {
        let summary = match self.intent {
            Some(text) if !text.trim().is_empty() => text,
            _ => anyhow::bail!("--intent est obligatoire : l'intention déclarée borne le pare-feu"),
        };

        let mut min_in = Vec::new();
        for raw in &self.min_in {
            let (token, amount) = raw
                .split_once(':')
                .ok_or_else(|| anyhow::anyhow!("--min-in attend <jeton>:<montant>"))?;
            let parsed = if token.to_uppercase() == "TEST" {
                parse_token_amount(amount, 18)?
            } else {
                parse_near(amount)?
            };
            min_in.push((token.to_string(), parsed));
        }

        Ok(Intent {
            summary,
            beneficiaries: self.beneficiaries,
            min_in,
            max_out: None,
        })
    }
}

/// Analyse la ligne d'arguments (sans le nom du programme).
pub fn parse(args: &[String]) -> anyhow::Result<Command> {
    let command = match args.first().map(String::as_str) {
        Some("propose") => {
            let opts = ProposeOptions::parse(&args[1..])?;
            let demand = opts.clone().into_demand()?;
            let intent = opts.into_intent()?;
            Command::Propose { demand, intent }
        }
        Some("approve") => match args.get(1) {
            Some(contract) => Command::Approve {
                contract: contract.clone(),
            },
            None => anyhow::bail!("usage : dryrun-gate approve <contrat>"),
        },
        Some("freeze") => Command::Freeze,
        Some("status") => Command::Status,
        Some("log") => Command::Log,
        Some("demo") => Command::Demo,
        Some("attack") => Command::Attack,
        Some("serve") => {
            let (port, web_dir, network) = parse_serve(&args[1..])?;
            Command::Serve {
                port,
                web_dir,
                network,
            }
        }
        Some("sim") => {
            let seed = args
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(42);
            Command::Sim { seed }
        }
        Some("mcp") => {
            let (network, lang) = parse_mcp(&args[1..])?;
            Command::Mcp { network, lang }
        }
        Some("help") | Some("--help") | Some("-h") | None => Command::Help,
        Some(other) => anyhow::bail!("commande inconnue : {other} (essayer `help`)"),
    };
    Ok(command)
}

/// Analyse les options de `serve` : `--port`, `--web-dir`, `--network`, `--vault`.
fn parse_serve(args: &[String]) -> anyhow::Result<(u16, String, Network)> {
    let mut port = 8787u16;
    let mut web_dir = "web".to_string();
    let mut choice = NetworkChoice::default();
    let mut index = 0;
    while index + 1 < args.len() {
        match args[index].as_str() {
            "--port" => {
                port = args[index + 1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("port invalide : {}", args[index + 1]))?
            }
            "--web-dir" => web_dir = args[index + 1].clone(),
            "--vault" => choice.vault = Some(args[index + 1].clone()),
            "--network" => choice.network = Some(args[index + 1].clone()),
            other => anyhow::bail!("option inconnue pour serve : {other}"),
        }
        index += 2;
    }
    Ok((port, web_dir, choice.resolve()?))
}

/// Analyse les options de `mcp` : `--network`, `--vault`, `--lang`.
fn parse_mcp(args: &[String]) -> anyhow::Result<(Network, Lang)> {
    let mut choice = NetworkChoice::default();
    let mut lang = Lang::default();
    let mut index = 0;
    while index + 1 < args.len() {
        match args[index].as_str() {
            "--vault" => choice.vault = Some(args[index + 1].clone()),
            "--lang" => lang = Lang::parse(&args[index + 1])?,
            "--network" => choice.network = Some(args[index + 1].clone()),
            other => anyhow::bail!("option inconnue pour mcp : {other}"),
        }
        index += 2;
    }
    Ok((choice.resolve()?, lang))
}

/// Choix de réseau fait par l'utilisateur, avant résolution.
#[derive(Clone, Debug, Default)]
struct NetworkChoice {
    /// Réseau demandé (`local` ou `testnet`).
    network: Option<String>,
    /// Compte du coffre demandé.
    vault: Option<String>,
}

impl NetworkChoice {
    /// Résout le choix en réseau utilisable.
    ///
    /// `--network testnet` exige `--vault` : sans compte de coffre, on ne peut
    /// ni lire `status()` ni signer quoi que ce soit.
    fn resolve(&self) -> anyhow::Result<Network> {
        let name = self.network.as_deref().unwrap_or("local");
        match name {
            "local" => Ok(Network::local()),
            "testnet" => match self.vault.as_deref() {
                Some(account) => Ok(Network::testnet(account.to_string())),
                None => anyhow::bail!(
                    "--network testnet exige --vault <compte.testnet> (le compte qui porte le coffre)"
                ),
            },
            other => anyhow::bail!("réseau inconnu : {other} (attendus : local, testnet)"),
        }
    }
}
