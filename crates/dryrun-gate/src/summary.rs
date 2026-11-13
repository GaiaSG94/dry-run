//! Ligne de résumé machine-lisible, en fin de chaque scénario CLI.
//!
//! Un script, la CI ou un jury peut vérifier les chiffres annoncés dans le
//! README sans lire la prose. Le simulateur actif est toujours nommé :
//! `backend=full` (bac à sable NEAR réel) ou `backend=stub` (bouchon mémoire).
//!
//! ```text
//! SUMMARY attack traps=7 stopped=7 rules_ok=7 backend=full
//! ```

use crate::gate::Gatekeeper;

/// Nom du simulateur actif : `full` (bac à sable NEAR réel) ou `stub`
/// (bouchon en mémoire, quand `near-sandbox` n'est pas disponible).
pub fn backend(gate: &Gatekeeper) -> &'static str {
    if gate.simulator().is_stub() {
        "stub"
    } else {
        "full"
    }
}

/// Affiche une ligne `SUMMARY`.
///
/// Une seule ligne, sans préfixe ni décoration : c'est la ligne qu'un script
/// va chercher au mot-clé `SUMMARY`.
pub fn line(command: &str, champs: &[(&str, String)]) {
    let mut out = format!("SUMMARY {command}");
    for (cle, valeur) in champs {
        out.push_str(&format!(" {cle}={valeur}"));
    }
    println!("{out}");
}
