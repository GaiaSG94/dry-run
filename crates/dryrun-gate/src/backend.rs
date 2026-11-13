//! Sélection du simulateur : vrai `dryrun-sim`, bouchon en mémoire, ou
//! coffre déployé sur le testnet public NEAR.
//!
//! - feature `real-sim` (**défaut**) : vraie simulation NEAR via `dryrun-sim`
//!   (section 4 du SPEC), adaptée par `crate::sim_real` ;
//! - feature `stub` : simulateur en mémoire `crate::sim_stub`, pour compiler et
//!   tester le crate seul, sans `near-sandbox` (`--no-default-features
//!   --features stub`) ;
//! - `crate::testnet` : le coffre `vault` réellement déployé sur le testnet,
//!   branché en lecture par RPC et en écriture par la CLI NEAR.
//!
//! Les trois implémentations sont toujours compilées : le garde-barrière
//! choisit à l'exécution, ce qui permet aux tests HTTP de tourner sur le
//! bouchon même quand la version par défaut est le vrai simulateur.

/// Déploie l'appel sur la variante active du simulateur.
macro_rules! dispatch {
    ($self:expr, $sim:ident => $call:expr) => {
        match $self {
            Simulator::Stub($sim) => $call,
            #[cfg(feature = "real-sim")]
            Simulator::Real($sim) => $call,
            Simulator::Testnet($sim) => $call,
        }
    };
}

use std::collections::BTreeMap;
use std::path::Path;

use dryrun_core::types::{Decision, Demande, Effects, Intent, SignedCert};

/// Rapport de la simulation différentielle, commun aux deux simulateurs.
pub type DifferentialReport = crate::sim_types::DifferentialReport;

#[cfg(feature = "real-sim")]
use crate::sim_real;
use crate::sim_stub;
pub use crate::sim_types::{Deployed, ExecOutcome, VaultStatus};
use crate::testnet;

/// Le simulateur utilisé par le garde-barrière.
#[allow(clippy::large_enum_variant)]
pub enum Simulator {
    /// Bouchon en mémoire : aucun contrat, aucune transaction réelle.
    Stub(sim_stub::Simulator),
    /// Vrai simulateur NEAR (`dryrun-sim`).
    #[cfg(feature = "real-sim")]
    Real(sim_real::Simulator),
    /// Coffre déployé sur le testnet public NEAR.
    Testnet(testnet::Simulator),
}

/// Ce qu'une exécution a donné : le résultat, le certificat éventuel et
/// l'empreinte de transaction (uniquement sur une vraie chaîne).
#[derive(Clone, Debug)]
pub struct ExecutionReport {
    /// Résultat de l'exécution.
    pub outcome: ExecOutcome,
    /// Certificat de simulation joint à la transaction (V1).
    pub cert: Option<SignedCert>,
    /// Vrai si la transaction est partie sur une chaîne publique.
    pub on_chain: bool,
    /// Empreinte de la transaction, sur une vraie chaîne.
    pub tx_hash: Option<String>,
}

impl Simulator {
    /// Démarre le simulateur prévu par la compilation.
    pub async fn start(wasm_dir: &Path) -> anyhow::Result<Self> {
        #[cfg(feature = "real-sim")]
        {
            // Repli explicite : si la chaîne locale n'est pas disponible (contrats
            // absents, bac à sable indisponible), la page et la CLI restent
            // utilisables avec le bouchon — et l'état affiché le dit clairement.
            match sim_real::Simulator::start(wasm_dir).await {
                Ok(real) => Ok(Self::Real(real)),
                Err(err) => {
                    eprintln!(
                        "attention : simulateur NEAR indisponible ({err:#}) — repli sur le bouchon en mémoire"
                    );
                    Self::stub().await
                }
            }
        }
        #[cfg(not(feature = "real-sim"))]
        {
            let _ = wasm_dir;
            Ok(Self::Stub(
                sim_stub::Simulator::start(Path::new("stub")).await?,
            ))
        }
    }

    /// Démarre explicitement le bouchon (tests, démonstration hors chaîne).
    pub async fn stub() -> anyhow::Result<Self> {
        Ok(Self::Stub(
            sim_stub::Simulator::start(Path::new("stub")).await?,
        ))
    }

    /// Démarre le coffre déployé sur le testnet public NEAR.
    pub async fn testnet(vault: &str, network: &str, rpc_url: &str) -> anyhow::Result<Self> {
        Ok(Self::Testnet(
            testnet::Simulator::start(vault, network, rpc_url).await?,
        ))
    }

    /// Vrai si ce simulateur est le bouchon.
    pub fn is_stub(&self) -> bool {
        match self {
            Self::Stub(_) => true,
            #[cfg(feature = "real-sim")]
            Self::Real(_) => false,
            Self::Testnet(_) => false,
        }
    }

    /// Vrai si ce simulateur pilote un coffre sur une vraie chaîne.
    pub fn is_testnet(&self) -> bool {
        match self {
            Self::Stub(_) => false,
            #[cfg(feature = "real-sim")]
            Self::Real(_) => false,
            Self::Testnet(_) => true,
        }
    }

    /// Déploie les contrats de démonstration.
    pub async fn deploy_all(&self) -> anyhow::Result<Deployed> {
        dispatch!(self, sim => sim.deploy_all().await)
    }

    /// Simule une demande et rend la décision.
    pub async fn simulate(&self, demand: &Demande, intent: &Intent) -> anyhow::Result<Decision> {
        dispatch!(self, sim => sim.simulate(demand, intent).await)
    }

    /// Simule deux fois (montant réel et centième) et rend décision + rapport.
    pub async fn simulate_differential(
        &self,
        demand: &Demande,
        intent: &Intent,
    ) -> anyhow::Result<(Decision, DifferentialReport)> {
        dispatch!(self, sim => sim.simulate_differential(demand, intent).await)
    }

    /// Clé publique du simulateur (hexadécimal), pour le coffre.
    pub fn sim_public_key(&self) -> String {
        match self {
            Self::Stub(sim) => sim.sim_public_key(),
            #[cfg(feature = "real-sim")]
            Self::Real(sim) => sim.sim_public_key(),
            Self::Testnet(sim) => sim.sim_public_key(),
        }
    }

    /// Le propriétaire enregistre la clé du simulateur sur le coffre (V1).
    pub async fn install_sim_key(&self) -> anyhow::Result<()> {
        dispatch!(self, sim => sim.install_sim_key().await)
    }

    /// Signe un certificat pour une demande déjà simulée (V1).
    pub async fn sign_cert(
        &self,
        demand: &Demande,
        effects: &Effects,
        post: Vec<(String, u128)>,
    ) -> anyhow::Result<SignedCert> {
        dispatch!(self, sim => sim.sign_cert(demand, effects, post).await)
    }

    /// Exécute réellement une demande sur la chaîne.
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome> {
        dispatch!(self, sim => sim.execute(demand).await)
    }

    /// Exécute une demande avec son certificat signé (V1).
    pub async fn execute_with_cert(
        &self,
        demand: &Demande,
        cert: Option<&SignedCert>,
    ) -> anyhow::Result<ExecOutcome> {
        dispatch!(self, sim => sim.execute_with_cert(demand, cert).await)
    }

    /// État du coffre.
    pub async fn vault_status(&self) -> anyhow::Result<VaultStatus> {
        dispatch!(self, sim => sim.vault_status().await)
    }

    /// Contrats connus du coffre.
    pub async fn known_contracts(&self) -> anyhow::Result<BTreeMap<String, String>> {
        dispatch!(self, sim => sim.known_contracts().await)
    }

    /// Gèle le coffre (clé de gel).
    pub async fn freeze(&self) -> anyhow::Result<()> {
        dispatch!(self, sim => sim.freeze().await)
    }

    /// Appel d'une méthode par le propriétaire (geste du propriétaire).
    pub async fn owner_call(
        &self,
        contract: &str,
        method: &str,
        args: &serde_json::Value,
    ) -> anyhow::Result<()> {
        dispatch!(self, sim => sim.owner_call(contract, method, args).await.map(|_| ()))
    }

    /// Approuve un contrat (clé du propriétaire).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        dispatch!(self, sim => sim.approve_contract(account, code_hash).await)
    }

    /// Exécute une demande autorisée, en signant si le backend le demande.
    ///
    /// Sur le testnet, aucune `sim_key` n'est enregistrée sur le coffre : le
    /// certificat de simulation n'y est pas exigé, l'action part avec
    /// `cert: null`, signée par la clé du compte via la CLI NEAR.
    pub async fn execute_approved(
        &self,
        demand: &Demande,
        intent: &Intent,
        effects: &Effects,
    ) -> anyhow::Result<ExecutionReport> {
        if self.is_testnet() {
            let outcome = self.execute(demand).await?;
            let tx_hash = Some(outcome.tx_hash.clone());
            return Ok(ExecutionReport {
                outcome,
                cert: None,
                on_chain: true,
                tx_hash,
            });
        }
        let post = crate::gate::post_conditions(intent);
        let cert = self.sign_cert(demand, effects, post).await?;
        let outcome = self.execute_with_cert(demand, Some(&cert)).await?;
        Ok(ExecutionReport {
            outcome,
            cert: Some(cert),
            on_chain: false,
            tx_hash: None,
        })
    }
}

/// Vrai si le crate a été compilé avec le vrai simulateur.
pub const IS_REAL_SIM: bool = cfg!(feature = "real-sim");
