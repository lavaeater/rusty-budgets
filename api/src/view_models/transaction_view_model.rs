use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::{BankTransaction, Money};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionViewModel {
    pub tx_id: Uuid,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub actual_item_id: Option<Uuid>,
}

impl TransactionViewModel {
    pub fn from_transaction(tx: &BankTransaction) -> Self {
        Self {
            tx_id: tx.id,
            amount: tx.amount,
            description: tx.description.clone(),
            date: tx.date,
            actual_item_id: tx.actual_id,
        }
    }
}