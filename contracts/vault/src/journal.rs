//! Journal du coffre : anneau de 100 entrées, lu par la page web et la CLI.

use crate::consts::LOG_CAPACITY;
use crate::types::LogEntry;
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::json_types::U128;
use near_sdk::AccountId;
use std::collections::VecDeque;

/// Anneau des dernières entrées du journal.
#[derive(Clone, Debug, Default, BorshSerialize, BorshDeserialize)]
pub struct Journal {
    entries: VecDeque<LogEntry>,
    next_seq: u64,
}

impl Journal {
    /// Ajoute une entrée ; la plus ancienne sort quand l'anneau est plein.
    pub fn push(
        &mut self,
        kind: &str,
        receiver: Option<&AccountId>,
        amount: u128,
        reason: Option<&str>,
        at: u64,
    ) {
        let entry = LogEntry {
            seq: U128(self.next_seq.into()),
            at: U128(at.into()),
            kind: kind.to_string(),
            receiver: receiver.cloned(),
            amount: U128(amount),
            reason: reason.map(|r| r.to_string()),
        };
        self.next_seq = self.next_seq.saturating_add(1);
        if self.entries.len() == LOG_CAPACITY {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Entrées dont le numéro d'ordre est au moins `from`, au plus `limit`.
    pub fn entries_from(&self, from: u64, limit: u32) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.seq.0 >= u128::from(from))
            .take(limit.min(LOG_CAPACITY as u32) as usize)
            .cloned()
            .collect()
    }

    /// Nombre d'entrées conservées.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
