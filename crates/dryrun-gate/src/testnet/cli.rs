//! Appel de la CLI NEAR pour signer et envoyer une transaction.
//!
//! La clé privée du compte vit dans le **trousseau du système** : elle n'est
//! jamais lue, ni affichée, ni copiée par Dry Run. La CLI la récupère elle-même
//! (`sign-with-keychain`) et signe la transaction.
//!
//! Le binaire est cherché dans l'ordre : variable d'environnement
//! `NEAR_CLI_BIN`, puis `near` dans le `PATH`, puis les emplacements usuels
//! (npm global, cargo). Sous Windows, l'enveloppe `near.cmd` de npm est
//! exécutée via `cmd /C` : `CreateProcess` ne sait pas lancer un script.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Variable d'environnement qui pointe sur l'exécutable de la CLI NEAR.
pub const CLI_ENV: &str = "NEAR_CLI_BIN";

/// Résultat d'un appel de contrat signé et envoyé sur la chaîne.
#[derive(Clone, Debug)]
pub struct CliCall {
    /// Empreinte de la transaction, telle qu'affichée par la CLI.
    pub tx_hash: String,
    /// Sortie complète de la CLI (diagnostic).
    pub output: String,
}

/// La CLI NEAR, prête à signer.
#[derive(Clone, Debug)]
pub struct NearCli {
    bin: PathBuf,
}

impl NearCli {
    /// Retrouve l'exécutable de la CLI NEAR.
    pub fn locate() -> anyhow::Result<Self> {
        if let Some(path) = std::env::var_os(CLI_ENV) {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(Self { bin: path });
            }
            anyhow::bail!(
                "{CLI_ENV} pointe sur un fichier absent : {}",
                path.display()
            );
        }
        // Le paquet npm range le vrai binaire dans `.bin_real` : l'appeler
        // directement évite de passer par Node à chaque transaction.
        if let Some(found) = npm_real_binary() {
            return Ok(Self { bin: found });
        }
        if let Some(found) = search_in_path("near") {
            return Ok(Self { bin: found });
        }
        for candidate in usual_locations() {
            if candidate.exists() {
                return Ok(Self { bin: candidate });
            }
        }
        anyhow::bail!(
            "CLI NEAR introuvable : installez near-cli-rs (https://github.com/near/near-cli-rs) \
             ou indiquez son chemin dans {CLI_ENV}"
        )
    }

    /// Chemin de l'exécutable trouvé.
    pub fn path(&self) -> &Path {
        &self.bin
    }

    /// Appelle une méthode du contrat en transaction, signée par le trousseau.
    ///
    /// `cert` n'est pas transmis : le coffre testnet n'a aucune `sim_key`
    /// enregistrée, donc le certificat V1 n'y est pas exigé.
    pub fn call_function(
        &self,
        contract: &str,
        method: &str,
        args: &Value,
        deposit: &str,
        signer: &str,
        network: &str,
    ) -> anyhow::Result<CliCall> {
        let args_text = serde_json::to_string(args)?;
        let output = self.run(&[
            "contract",
            "call-function",
            "as-transaction",
            contract,
            method,
            "json-args",
            &args_text,
            "prepaid-gas",
            "100 Tgas",
            "attached-deposit",
            deposit,
            "sign-as",
            signer,
            "network-config",
            network,
            "sign-with-keychain",
            "send",
        ])?;
        let tx_hash = parse_tx_hash(&output).ok_or_else(|| {
            anyhow::anyhow!("la CLI NEAR n'a pas renvoyé d'empreinte de transaction :\n{output}")
        })?;
        Ok(CliCall { tx_hash, output })
    }

    /// Exécute la CLI et rend sa sortie complète (stdout + stderr).
    fn run(&self, args: &[&str]) -> anyhow::Result<String> {
        let output = self
            .spawn(args)
            .map_err(|err| anyhow::anyhow!("lancement de {} : {err}", self.bin.display()))?;
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push('\n');
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        if !output.status.success() {
            anyhow::bail!(
                "la CLI NEAR a échoué (code {:?}) :\n{}",
                output.status.code(),
                tail(&text)
            );
        }
        Ok(text)
    }

    /// Lance la CLI, en passant par `cmd /C` pour les scripts Windows.
    fn spawn(&self, args: &[&str]) -> std::io::Result<std::process::Output> {
        let extension = self
            .bin
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if cfg!(windows) && (extension == "cmd" || extension == "bat") {
            let mut command = Command::new("cmd");
            command.arg("/C").arg(&self.bin);
            command.args(args);
            return command.output();
        }
        Command::new(&self.bin).args(args).output()
    }
}

/// Extrait l'empreinte de transaction annoncée par la CLI.
fn parse_tx_hash(output: &str) -> Option<String> {
    let after = output.split("Transaction ID:").nth(1)?;
    let hash: String = after
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric())
        .collect();
    if hash.len() >= 32 {
        Some(hash)
    } else {
        None
    }
}

/// Les `n` dernières lignes d'un texte, pour un message d'erreur lisible.
fn tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(25);
    lines[start..].join("\n")
}

/// Cherche un exécutable dans le `PATH`.
fn search_in_path(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let extensions: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ".bat", ""]
    } else {
        &[""]
    };
    for dir in std::env::split_paths(&path) {
        for extension in extensions {
            let candidate = dir.join(format!("{name}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// Emplacements usuels de la CLI NEAR, quand elle n'est pas dans le `PATH`.
fn usual_locations() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA") {
        paths.push(PathBuf::from(appdata).join("npm").join("near.cmd"));
    }
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let cargo = PathBuf::from(&home).join(".cargo").join("bin");
        paths.push(cargo.join("near"));
        if cfg!(windows) {
            paths.push(cargo.join("near.exe"));
        }
    }
    paths
}

/// Le vrai binaire du paquet npm global (sans l'enveloppe Node).
fn npm_real_binary() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let appdata = std::env::var_os("APPDATA")?;
    let candidate = PathBuf::from(appdata)
        .join("npm")
        .join("node_modules")
        .join("near-cli-rs")
        .join("node_modules")
        .join(".bin_real")
        .join("near.exe");
    candidate.is_file().then_some(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empreinte_extraite_de_la_sortie_cli() {
        let output = "│    Transaction ID: 8rcyKSL7RkZ1qJjomBJu1aY7JPrSUKSgYQLcs177iDXv\n│    url";
        assert_eq!(
            parse_tx_hash(output).as_deref(),
            Some("8rcyKSL7RkZ1qJjomBJu1aY7JPrSUKSgYQLcs177iDXv")
        );
    }

    #[test]
    fn empreinte_absente_renvoie_none() {
        assert!(parse_tx_hash("rien à signaler ici").is_none());
    }
}
