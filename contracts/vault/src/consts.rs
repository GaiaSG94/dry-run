//! Constantes du coffre : codes de refus, délais et bornes de l'état.

// ---------------------------------------------------------------- refus ---
/// Le coffre est gelé : aucune sortie n'est possible.
pub const ERR_FROZEN: &str = "ERR_FROZEN";
/// Le montant dépasse le plafond par transaction.
pub const ERR_OVER_TX_CAP: &str = "ERR_OVER_TX_CAP";
/// Le montant dépasse le reste du plafond sur 24 heures glissantes.
pub const ERR_OVER_DAILY_CAP: &str = "ERR_OVER_DAILY_CAP";
/// Appel vers un compte absent de la liste des contrats connus.
pub const ERR_UNKNOWN_CONTRACT: &str = "ERR_UNKNOWN_CONTRACT";
/// Contrat marqué comme hostile par le propriétaire.
pub const ERR_HOSTILE: &str = "ERR_HOSTILE";
/// Le contrat visé n'a pas produit de certificat de simulation.
pub const ERR_NO_CERT: &str = "ERR_NO_CERT";
/// Le certificat de simulation est trop ancien.
pub const ERR_CERT_STALE: &str = "ERR_CERT_STALE";
/// Le certificat ne correspond pas à l'action proposée.
pub const ERR_CERT_MISMATCH: &str = "ERR_CERT_MISMATCH";
/// Le certificat a déjà servi (protection contre le rejeu).
pub const ERR_CERT_USED: &str = "ERR_CERT_USED";
/// Signature du certificat invalide.
pub const ERR_BAD_SIGNATURE: &str = "ERR_BAD_SIGNATURE";
/// Action trop rapprochée d'un changement précédent (délai de sécurité).
pub const ERR_COOLDOWN: &str = "ERR_COOLDOWN";

// ------------------------------------------------------- autorisation ----
/// Le compte appelant n'a pas le droit demandé (I4, I7).
pub const ERR_UNAUTHORIZED: &str = "ERR_UNAUTHORIZED";
/// L'action exige exactement 1 yoctoNEAR de dépôt.
pub const ERR_ONE_YOCTO: &str = "ERR_ONE_YOCTO";

// -------------------------------------------------------------- délais ---
/// Délai minimal entre le gel et le dégel : 1 heure.
pub const UNFREEZE_DELAY_NS: u64 = 3_600_000_000_000;
/// Délai minimal avant de desserrer un plafond : 1 heure.
pub const CAPS_DELAY_NS: u64 = 3_600_000_000_000;

// --------------------------------------------------------------- bornes ---
/// Nombre d'entrées gardées dans l'anneau du journal.
pub const LOG_CAPACITY: usize = 100;
/// Nombre de clés observées mémorisées par catégorie.
pub const MAX_TRACKED_KEYS: usize = 8;
/// Gas statique du rappel `on_done`, en Tgas.
pub const CALLBACK_GAS_TGAS: u64 = 10;
/// Dépôt exigé par les actions du propriétaire : 1 yoctoNEAR.
pub const ONE_YOCTO: u128 = 1;
