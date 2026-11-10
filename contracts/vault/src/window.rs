//! Fenêtre glissante de 24 heures, découpée en 24 cases horaires (invariant I1).
//!
//! Une case retient la dépense d'une heure pleine et la date de cette heure.
//! La dépense courante est la somme des cases datant de moins de 24 heures :
//! la fenêtre glisse donc d'heure en heure sans jamais balayer tout le tableau.

use near_sdk::borsh::{BorshDeserialize, BorshSerialize};

/// Nombre de cases horaires : la fenêtre fait 24 heures.
pub const SLOTS: usize = 24;
/// Durée d'une heure en nanosecondes.
pub const HOUR_NS: u64 = 3_600_000_000;

/// Dépenses horaires des dernières 24 heures.
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Window {
    /// Montants dépensés, une case par heure modulo 24.
    spent: [u128; SLOTS],
    /// Heure de la case, plus un (0 = case jamais utilisée).
    stamp: [u64; SLOTS],
}

impl Window {
    /// Case horaire d'une heure donnée.
    pub fn slot(hour: u64) -> usize {
        (hour % SLOTS as u64) as usize
    }

    /// Comptabilise `amount` sur l'heure `hour`.
    pub fn add(&mut self, hour: u64, amount: u128) {
        let slot = Self::slot(hour);
        if self.stamp[slot] != hour.saturating_add(1) {
            self.stamp[slot] = hour.saturating_add(1);
            self.spent[slot] = 0;
        }
        self.spent[slot] = self.spent[slot].saturating_add(amount);
    }

    /// Annule `amount` comptabilisé sur l'heure `hour` (le reçu a échoué).
    ///
    /// Si la case a été recyclée depuis (plus de 24 heures), l'annulation est
    /// sans effet : le montant est de toute façon sorti de la fenêtre.
    pub fn rollback(&mut self, hour: u64, amount: u128) {
        let slot = Self::slot(hour);
        if self.stamp[slot] == hour.saturating_add(1) {
            self.spent[slot] = self.spent[slot].saturating_sub(amount);
        }
    }

    /// Total dépensé sur les 24 heures qui précèdent `hour`, incluse.
    pub fn total(&self, hour: u64) -> u128 {
        let mut sum = 0u128;
        for slot in 0..SLOTS {
            if self.is_fresh(slot, hour) {
                sum = sum.saturating_add(self.spent[slot]);
            }
        }
        sum
    }

    /// Une case est fraîche si son heure appartient aux 24 dernières heures.
    fn is_fresh(&self, slot: usize, hour: u64) -> bool {
        match self.stamp[slot] {
            0 => false,
            stamp => stamp - 1 + SLOTS as u64 > hour,
        }
    }
}
