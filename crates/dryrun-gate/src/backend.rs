//! Sélection du simulateur : vrai `dryrun-sim` ou bouchon en mémoire.
//!
//! - feature `real-sim` (**défaut**) : vraie simulation NEAR via `dryrun-sim`
//!   (section 4 du SPEC), adaptée par `crate::sim_real` ;
//! - feature `stub` : simulateur en mémoire `crate::sim_stub`, pour compiler et
//!   tester le crate seul, sans `near-sandbox` (`--no-default-features
//!   --features stub`).
//!
//! Les deux implémentations sont toujours compilées : le garde-barrière choisit
//! à l'exécution, ce qui permet aux tests HTTP de tourner sur le bouchon même
//! quand la version par défaut est le vrai simulateur.

/// Déploie l'appel sur la variante active du simulateur.
macro_rules! dispatch {
    ($self:expr, $sim:ident => $call:expr) => {
        match $self {
            Simulator::Stub($sim) => $call,
            #[cfg(feature = "real-sim")]
            Simulator::Real($sim) => $call,
        }
    };
}

use std::collections::BTreeMap;
use std::path::Path;

use dryrun_core::types::{Decision, Demande, Intent};

#[cfg(feature = "real-sim")]
use crate::sim_real;
use crate::sim_stub;
pub use crate::sim_types::{Deployed, ExecOutcome, VaultStatus};

#[cfg(all(feature = "stub", feature = "real-sim"))]
compile_error!("les features `stub` et `real-sim` s'excluent : choisir un seul simulateur");

/// Le simulateur utilisé par le garde-barrière.
#[allow(clippy::large_enum_variant)]
pub enum Simulator {
    /// Bouchon en mémoire : aucun contrat, aucune transaction réelle.
    Stub(sim_stub::Simulator),
    /// Vrai simulateur NEAR (`dryrun-sim`).
    #[cfg(feature = "real-sim")]
    Real(sim_real::Simulator),
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

    /// Vrai si ce simulateur est le bouchon.
    pub fn is_stub(&self) -> bool {
        match self {
            Self::Stub(_) => true,
            #[cfg(feature = "real-sim")]
            Self::Real(_) => false,
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

    /// Exécute réellement une demande sur la chaîne.
    pub async fn execute(&self, demand: &Demande) -> anyhow::Result<ExecOutcome> {
        dispatch!(self, sim => sim.execute(demand).await)
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

    /// Approuve un contrat (clé du propriétaire).
    pub async fn approve_contract(&self, account: &str, code_hash: &str) -> anyhow::Result<()> {
        dispatch!(self, sim => sim.approve_contract(account, code_hash).await)
    }
}

/// Vrai si le crate a été compilé avec le vrai simulateur.
pub const IS_REAL_SIM: bool = cfg!(feature = "real-sim");
