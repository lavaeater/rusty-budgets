use crate::models::money::Money;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionAllocation {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub actual_id: Uuid,
    pub amount: Money,
    pub tag: String,
}

impl TransactionAllocation {
    pub fn new(transaction_id: Uuid, actual_id: Uuid, amount: Money, tag: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            transaction_id,
            actual_id,
            amount,
            tag,
        }
    }
}
