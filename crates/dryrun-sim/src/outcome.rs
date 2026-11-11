//! Lecture d'une exécution NEAR : reçus, journaux, gas.

use dryrun_core::ReceiptInfo;
use near_primitives::views::{
    ExecutionStatusView, FinalExecutionOutcomeView, FinalExecutionStatus,
};

/// Une exécution résumée, partagée par la chaîne et le simulateur.
#[derive(Clone, Debug, Default)]
pub struct Execution {
    pub success: bool,
    pub error: Option<String>,
    pub receipts: Vec<ReceiptInfo>,
    pub logs: Vec<String>,
    pub gas_burnt: u64,
}

impl Execution {
    /// Identifiants des codes d'erreur repérés dans le message d'échec.
    pub fn error_codes(&self) -> Vec<String> {
        let Some(error) = &self.error else {
            return Vec::new();
        };
        error
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .filter(|word| word.starts_with("ERR_"))
            .map(|word| word.to_string())
            .collect()
    }

    /// Code de refus explicite : événement NEP-297 `refused`, puis message d'erreur.
    ///
    /// Le contrat journalise `{"standard":"nep297","event":"refused","data":{"reason":"ERR_..."}}`
    /// juste avant de paniquer : ces journaux survivent à l'échec du reçu, c'est
    /// donc la source la plus fiable (les messages de panique, eux, dépendent du
    /// profil de compilation).
    pub fn refusal(&self) -> Option<String> {
        for log in &self.logs {
            if let Some(reason) = refusal_in_log(log) {
                return Some(reason);
            }
        }
        self.error_codes().into_iter().next()
    }
}

/// Extrait `data.reason` d'un journal d'événement `refused`.
fn refusal_in_log(log: &str) -> Option<String> {
    let json = log.strip_prefix("EVENT_JSON:").unwrap_or(log);
    if !json.contains("refused") {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let reason = value
        .get("data")
        .and_then(|data| data.get("reason"))
        .and_then(|reason| reason.as_str())?;
    reason.starts_with("ERR_").then(|| reason.to_string())
}

/// Résume une exécution finale.
pub fn parse(outcome: &FinalExecutionOutcomeView) -> Execution {
    let mut logs = outcome.transaction_outcome.outcome.logs.clone();
    let mut gas_burnt = outcome.transaction_outcome.outcome.gas_burnt.as_gas();
    let receipts: Vec<ReceiptInfo> = outcome
        .receipts_outcome
        .iter()
        .map(|receipt| {
            logs.extend(receipt.outcome.logs.iter().cloned());
            gas_burnt += receipt.outcome.gas_burnt.as_gas();
            // Le message d'échec d'un reçu porte souvent la vraie cause
            // (panique du contrat, dépassement de gas) : on le garde.
            if let ExecutionStatusView::Failure(error) = &receipt.outcome.status {
                logs.push(format!("RECEIPT_ERROR: {error:?}"));
            }
            ReceiptInfo {
                id: receipt.id.to_string(),
                status: status_name(&receipt.outcome.status).to_string(),
                receiver: receipt.outcome.executor_id.to_string(),
                logs: receipt.outcome.logs.clone(),
            }
        })
        .collect();
    Execution {
        success: matches!(outcome.status, FinalExecutionStatus::SuccessValue(_)),
        error: match &outcome.status {
            FinalExecutionStatus::Failure(error) => Some(error.to_string()),
            _ => None,
        },
        receipts,
        logs,
        gas_burnt,
    }
}

/// Nom stable d'un statut de reçu : `success`, `failed` ou `unknown`.
fn status_name(status: &ExecutionStatusView) -> &'static str {
    match status {
        ExecutionStatusView::Failure(_) => "failed",
        ExecutionStatusView::SuccessValue(_) | ExecutionStatusView::SuccessReceiptId(_) => {
            "success"
        }
        ExecutionStatusView::Unknown => "unknown",
    }
}
