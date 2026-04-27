use crate::models::{BankTransaction, Money, strip_dates};
use crate::view_models::AllocationViewModel;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionViewModel {
    pub tx_id: Uuid,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub account_number: String,
    pub actual_item_id: Option<Uuid>,
    pub allocations: Vec<AllocationViewModel>,
}

impl TransactionViewModel {
    pub fn from_transaction(tx: &BankTransaction) -> Self {
        Self {
            tx_id: tx.id,
            amount: tx.amount,
            description: strip_dates(&tx.description),
            date: tx.date,
            account_number: tx.account_number.clone(),
            actual_item_id: tx.actual_id,
            allocations: Vec::new(),
        }
    }

    pub fn from_transaction_with_allocations(
        tx: &BankTransaction,
        allocations: Vec<AllocationViewModel>,
    ) -> Self {
        Self {
            allocations,
            ..Self::from_transaction(tx)
        }
    }
}
