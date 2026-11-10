//! Certificat de simulation signé (niveau V1).
//!
//! Le coffre n'exécute qu'une demande accompagnée d'un certificat **frais**,
//! **à usage unique** et **signé** par la clé du simulateur. Voler la clé de
//! l'agent ne suffit donc plus : il faut aussi celle du simulateur, qui ne
//! signe aucune transaction et ne peut rien dépenser seule.

use dryrun_core::types::{Cert, SignedCert};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

/// Signeur de certificats : une paire de clés Ed25519 détenue par le simulateur.
pub struct CertSigner {
    signing: SigningKey,
}

impl CertSigner {
    /// Génère une paire de clés neuve (une par processus de simulation).
    pub fn new() -> Self {
        CertSigner {
            signing: SigningKey::generate(&mut OsRng),
        }
    }

    /// Paire de clés dérivée d'une graine fixe (tests reproductibles).
    pub fn from_seed(seed: [u8; 32]) -> Self {
        CertSigner {
            signing: SigningKey::from_bytes(&seed),
        }
    }

    /// Clé publique au format hexadécimal, telle que le coffre la compare.
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.signing.verifying_key().to_bytes())
    }

    /// Signe un certificat et rend le couple certificat + signature.
    pub fn sign(&self, cert: Cert) -> SignedCert {
        let payload = canonical_bytes(&cert);
        let signature = self.signing.sign(&payload);
        SignedCert {
            cert,
            signature: hex::encode(signature.to_bytes()),
            public_key: self.public_key_hex(),
        }
    }
}

impl Default for CertSigner {
    fn default() -> Self {
        Self::new()
    }
}

/// Octets canoniques signés : la sérialisation JSON du certificat.
///
/// Le coffre recompute exactement ces octets à partir du certificat reçu,
/// d'où l'exigence d'une sérialisation stable (ordre des champs fixé par
/// l'ordre de déclaration dans `Cert`).
pub fn canonical_bytes(cert: &Cert) -> Vec<u8> {
    serde_json::to_vec(cert).unwrap_or_default()
}
