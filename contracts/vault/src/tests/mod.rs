//! Tests unitaires du coffre (faux bac à sable de near-sdk, sans réseau).
//!
//! Chaque fichier couvre un invariant du cahier des charges :
//! I1 `caps`, I2 `freeze`, I3 `known`, I4 et I7 `auth`, I5 `settlement`.

mod auth;
mod caps;
mod events;
mod freeze;
mod known;
mod serialization;
mod settlement;
mod setup;
