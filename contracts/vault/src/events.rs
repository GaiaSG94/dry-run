//! Événements NEP-297 du coffre.
//!
//! Format : `{"standard":"nep297","version":"1.0.0","event":"<nom>","data":{…}}`.
//! Ils sont écrits dans les journaux du reçu, donc visibles même quand la
//! transaction échoue (cas `refused`).

use near_sdk::{env, AccountId};
use serde_json::{json, Value};

/// Norme suivie par tous les événements.
pub const STANDARD: &str = "nep297";
/// Version du format.
pub const VERSION: &str = "1.0.0";

/// Écrit un événement NEP-297 dans le journal du reçu.
pub fn emit(event: &str, data: Value) {
    let payload = json!({
        "standard": STANDARD,
        "version": VERSION,
        "event": event,
        "data": data,
    });
    env::log_str(&payload.to_string());
}

/// Une sortie a été acceptée et l'appel a été envoyé.
pub fn executed(kind: &str, receiver: &AccountId, amount: u128) {
    emit(
        "executed",
        json!({ "kind": kind, "receiver": receiver, "amount": amount.to_string() }),
    );
}

/// Une action a été refusée ; `reason` est l'un des codes `ERR_*`.
pub fn refused(reason: &str) {
    emit("refused", json!({ "reason": reason }));
}

/// Le coffre vient d'être gelé.
pub fn frozen(at: u64) {
    emit("frozen", json!({ "at": at.to_string() }));
}

/// Le coffre vient d'être dégelé par le propriétaire.
pub fn unfrozen(at: u64) {
    emit("unfrozen", json!({ "at": at.to_string() }));
}

/// Un contrat est entré dans la liste des contrats connus.
pub fn contract_approved(account_id: &AccountId, code_hash: &str) {
    emit(
        "contract_approved",
        json!({ "account_id": account_id, "code_hash": code_hash }),
    );
}

/// Le reçu d'une sortie a échoué : la dépense comptée est annulée (I5).
pub fn postcondition_failed(kind: &str, receiver: &AccountId, amount: u128) {
    emit(
        "postcondition_failed",
        json!({ "kind": kind, "receiver": receiver, "amount": amount.to_string() }),
    );
}

/// Un compte a été marqué comme hostile (niveau V1, certificats de simulation).
///
/// Le contrat ne l'émet pas encore : le chemin des certificats (`ERR_HOSTILE`,
/// `ERR_NO_CERT`, `ERR_CERT_*`) n'est pas branché sur cette version.
pub fn hostile_marked(account_id: &AccountId, reason: &str) {
    emit(
        "hostile_marked",
        json!({ "account_id": account_id, "reason": reason }),
    );
}
