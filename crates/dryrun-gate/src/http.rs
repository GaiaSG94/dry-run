//! Serveur HTTP autonome et léger pour dryrun-gate.
//!
//! Sert les fichiers statiques de `web/` et expose l'API JSON. Aucun framework :
//! un `TcpListener` de la bibliothèque standard suffit pour une démonstration
//! locale, et cela évite toute dépendance réseau au moment de la démo.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use dryrun_core::types::{Demande, Intent, Verdict};
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::amount::Amount;
use crate::gate::Gatekeeper;

/// Gas par défaut si le client n'en fournit pas : 30 Tgas.
const DEFAULT_GAS: u64 = 30_000_000_000_000;

/// Taille maximale acceptée pour une requête (garde-fou anti-abus).
const MAX_REQUEST_BYTES: usize = 1 << 20;

/// Nature de la demande envoyée à `POST /api/propose`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DemandKind {
    /// Transfert direct de NEAR.
    Transfer,
    /// Appel de fonction sur un contrat, avec dépôt éventuel.
    FunctionCall,
}

/// Intention déclarée, telle qu'envoyée par la page web.
#[derive(Clone, Debug, Deserialize)]
pub struct IntentInput {
    /// Phrase lisible décrivant l'opération voulue.
    #[serde(default)]
    pub summary: String,
    /// Comptes autorisés à recevoir quelque chose.
    #[serde(default)]
    pub beneficiaries: Vec<String>,
    /// Jetons et montants minimaux attendus.
    #[serde(default)]
    pub min_in: Vec<(String, Amount)>,
    /// Plafond de sortie annoncé, s'il y en a un.
    #[serde(default)]
    pub max_out: Option<(String, Amount)>,
}

impl IntentInput {
    /// Convertit l'entrée JSON en `Intent` du domaine.
    fn into_intent(self) -> Intent {
        Intent {
            summary: self.summary,
            beneficiaries: self.beneficiaries,
            min_in: self
                .min_in
                .into_iter()
                .map(|(token, amount)| (token, amount.yocto()))
                .collect(),
            max_out: self
                .max_out
                .map(|(token, amount)| (token, amount.yocto())),
        }
    }
}

/// Corps accepté par `POST /api/propose`.
#[derive(Clone, Debug, Deserialize)]
pub struct ProposeRequest {
    /// `transfer` ou `function_call`.
    pub kind: DemandKind,
    /// Destinataire d'un transfert.
    #[serde(default)]
    pub to: Option<String>,
    /// Montant d'un transfert, en yoctoNEAR.
    #[serde(default)]
    pub amount: Amount,
    /// Contrat appelé.
    #[serde(default)]
    pub contract: Option<String>,
    /// Méthode appelée.
    #[serde(default)]
    pub method: Option<String>,
    /// Arguments JSON de l'appel.
    #[serde(default)]
    pub args: Option<serde_json::Value>,
    /// Dépôt joint, en yoctoNEAR.
    #[serde(default)]
    pub deposit: Amount,
    /// Gas alloué à l'appel.
    #[serde(default)]
    pub gas: Option<u64>,
    /// Intention déclarée, qui borne le pare-feu.
    pub intent: IntentInput,
    /// Confirmation explicite du propriétaire, utile pour un verdict `WARN`.
    #[serde(default)]
    pub confirm: bool,
}

impl ProposeRequest {
    /// Construit la demande et l'intention du domaine.
    fn into_parts(self) -> anyhow::Result<(Demande, Intent)> {
        let intent = self.intent.into_intent();
        let demand = match self.kind {
            DemandKind::Transfer => {
                let to = self
                    .to
                    .ok_or_else(|| anyhow::anyhow!("champ `to` manquant pour un transfert"))?;
                Demande::Transfer {
                    to,
                    amount: self.amount.yocto(),
                }
            }
            DemandKind::FunctionCall => {
                let contract = self
                    .contract
                    .ok_or_else(|| anyhow::anyhow!("champ `contract` manquant pour un appel"))?;
                let method = self
                    .method
                    .ok_or_else(|| anyhow::anyhow!("champ `method` manquant pour un appel"))?;
                Demande::FunctionCall {
                    contract,
                    method,
                    args: self.args.unwrap_or(serde_json::Value::Null),
                    deposit: self.deposit.yocto(),
                    gas: self.gas.unwrap_or(DEFAULT_GAS),
                }
            }
        };
        Ok((demand, intent))
    }
}

/// Serveur HTTP minimaliste du garde-barrière.
pub struct HttpServer {
    gate: Arc<Gatekeeper>,
    web_dir: PathBuf,
}

impl HttpServer {
    /// Prépare le serveur ; `web_dir` est lu à la volée à chaque requête.
    pub fn new(gate: Arc<Gatekeeper>, web_dir: PathBuf) -> Self {
        Self { gate, web_dir }
    }

    /// Écoute en boucle et traite chaque connexion dans sa propre tâche.
    pub async fn run(self: Arc<Self>, addr: &str) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Dry Run — interface sur http://{addr} (Ctrl-C pour arrêter)");
        loop {
            match listener.accept().await {
                Ok((stream, _peer)) => {
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(err) = server.handle(stream).await {
                            eprintln!("requête HTTP en erreur : {err}");
                        }
                    });
                }
                Err(err) => eprintln!("connexion refusée : {err}"),
            }
        }
    }

    /// Lit une requête, la route, puis écrit la réponse.
    async fn handle(&self, mut stream: TcpStream) -> anyhow::Result<()> {
        let raw = match read_request(&mut stream).await? {
            Some(text) => text,
            None => return Ok(()),
        };
        let (method, path) = match request_line(&raw) {
            Some(pair) => pair,
            None => return Ok(()),
        };
        let body = request_body(&raw);

        match (method.as_str(), path.as_str()) {
            ("GET", "/api/status") => {
                let status = self.gate.status().await?;
                send_json(&mut stream, 200, "OK", &status).await
            }
            ("GET", "/api/log") => {
                let entries = self.gate.journal().views();
                send_json(&mut stream, 200, "OK", &entries).await
            }
            ("POST", "/api/propose") => self.api_propose(&mut stream, &body).await,
            ("POST", "/api/freeze") => self.api_freeze(&mut stream).await,
            ("GET", _) => self.serve_static(&mut stream, &path).await,
            _ => send_error(&mut stream, 404, "route introuvable").await,
        }
    }

    /// `POST /api/propose` : simule, puis signe seulement si `ALLOW` (ou `WARN` confirmé).
    async fn api_propose(&self, stream: &mut TcpStream, body: &str) -> anyhow::Result<()> {
        let request: ProposeRequest = match serde_json::from_str(body) {
            Ok(req) => req,
            Err(err) => return send_error(stream, 400, &format!("JSON invalide : {err}")).await,
        };
        let confirm = request.confirm;
        let (demand, intent) = match request.into_parts() {
            Ok(parts) => parts,
            Err(err) => return send_error(stream, 400, &err.to_string()).await,
        };

        let mut outcome = self.gate.propose(&demand, &intent).await?;
        if outcome.verdict == Verdict::Warn && confirm {
            let (executed, note) = self.gate.confirm(&demand).await?;
            outcome.executed = executed;
            outcome.note = note;
        }
        send_json(stream, 200, "OK", &outcome).await
    }

    /// `POST /api/freeze` : gèle le coffre et rend le nouvel état.
    async fn api_freeze(&self, stream: &mut TcpStream) -> anyhow::Result<()> {
        self.gate.freeze().await?;
        let status = self.gate.status().await?;
        send_json(stream, 200, "OK", &status).await
    }

    /// Sert un fichier de `web/` (`index.html` par défaut pour `/`).
    async fn serve_static(&self, stream: &mut TcpStream, path: &str) -> anyhow::Result<()> {
        let relative = if path == "/" || path.is_empty() {
            "index.html"
        } else {
            path.trim_start_matches('/')
        };
        if relative.contains("..") {
            return send_error(stream, 403, "chemin interdit").await;
        }

        let file_path = self.web_dir.join(relative);
        match tokio::fs::read(&file_path).await {
            Ok(content) => {
                let mime = mime_for_path(&file_path);
                write_response(stream, 200, "OK", mime, &content).await
            }
            Err(_) => send_error(stream, 404, "fichier introuvable").await,
        }
    }
}

/// Lit une requête complète : en-têtes puis corps annoncé par `Content-Length`.
async fn read_request(stream: &mut TcpStream) -> anyhow::Result<Option<String>> {
    let mut buffer = [0u8; 8192];
    let mut raw = Vec::new();

    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        raw.extend_from_slice(&buffer[..read]);
        if let Some(expected) = expected_total(&raw) {
            if raw.len() >= expected {
                break;
            }
        }
        if raw.len() > MAX_REQUEST_BYTES {
            break;
        }
    }

    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&raw).to_string()))
}

/// Taille totale attendue (en-têtes + corps), d'après `Content-Length`.
fn expected_total(raw: &[u8]) -> Option<usize> {
    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n")? + 4;
    let headers = String::from_utf8_lossy(&raw[..header_end]).to_lowercase();
    let length = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length:"))
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    Some(header_end + length)
}

/// Extrait la méthode et le chemin (sans query string) de la première ligne.
fn request_line(raw: &str) -> Option<(String, String)> {
    let first = raw.lines().next()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let path = target.split('?').next().unwrap_or(&target).to_string();
    Some((method, path))
}

/// Extrait le corps de la requête (après la ligne vide séparatrice).
fn request_body(raw: &str) -> String {
    match raw.find("\r\n\r\n") {
        Some(pos) => raw[pos + 4..].to_string(),
        None => String::new(),
    }
}

/// Écrit une réponse JSON complète.
async fn send_json<T: serde::Serialize>(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    data: &T,
) -> anyhow::Result<()> {
    let body = serde_json::to_vec(data)?;
    write_response(stream, status, reason, "application/json; charset=utf-8", &body).await
}

/// Écrit une réponse d'erreur JSON normalisée.
async fn send_error(stream: &mut TcpStream, status: u16, message: &str) -> anyhow::Result<()> {
    let payload = serde_json::json!({ "error": message });
    let reason = match status {
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    send_json(stream, status, reason, &payload).await
}

/// Écrit les en-têtes puis le corps, et ferme la connexion.
async fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) -> anyhow::Result<()> {
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

/// Type MIME déduit de l'extension du fichier.
fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") | Some("htm") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    }
}