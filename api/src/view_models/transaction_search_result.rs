use crate::models::BankTransaction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionSearchResult {
    pub transactions: Vec<BankTransaction>,
    /// Total number of transactions matching the filter, before `limit`/`offset` —
    /// drives the "N transaktioner" label and whether another page exists.
    pub total_count: usize,
}
