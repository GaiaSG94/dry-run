//! Analyse des arguments de la ligne de commande, à la main (sans `clap`).

use dryrun_core::types::{Demande, Intent, Yocto};

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
    Serve { port: u16, web_dir: String },
    /// Lance le banc de test automatisé de 200 robots (matrice de confusion).
    Sim { seed: u64 },
    /// Démarre le serveur MCP JSON-RPC sur stdin/stdout pour les agents IA.
    Mcp,
    /// Affiche l'aide.
    Help,
}

/// Convertit un montant NEAR décimal en yoctoNEAR (1 NEAR = 10^24 yocto).
pub fn parse_near(text: &str) -> anyhow::Result<Yocto> {
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
    frac.truncate(24);
    while frac.len() < 24 {
        frac.push('0');
    }
    let frac: u128 = if frac.is_empty() {
        0
    } else {
        frac.parse()
            .map_err(|_| anyhow::anyhow!("montant invalide : {trimmed}"))?
    };
    Ok(whole * 10u128.pow(24) + frac)
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
            min_in.push((token.to_string(), parse_near(amount)?));
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
            let (port, web_dir) = parse_serve(&args[1..])?;
            Command::Serve { port, web_dir }
        }
        Some("sim") => {
            let seed = args
                .get(1)
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(42);
            Command::Sim { seed }
        }
        Some("mcp") => Command::Mcp,
        Some("help") | Some("--help") | Some("-h") | None => Command::Help,
        Some(other) => anyhow::bail!("commande inconnue : {other} (essayer `help`)"),
    };
    Ok(command)
}

/// Analyse les options de `serve` : `--port` et `--web-dir`.
fn parse_serve(args: &[String]) -> anyhow::Result<(u16, String)> {
    let mut port = 8787u16;
    let mut web_dir = "web".to_string();
    let mut index = 0;
    while index + 1 < args.len() {
        match args[index].as_str() {
            "--port" => {
                port = args[index + 1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("port invalide : {}", args[index + 1]))?
            }
            "--web-dir" => web_dir = args[index + 1].clone(),
            other => anyhow::bail!("option inconnue pour serve : {other}"),
        }
        index += 2;
    }
    Ok((port, web_dir))
}
