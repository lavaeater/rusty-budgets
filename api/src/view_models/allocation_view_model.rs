use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{Money, TransactionAllocation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AllocationViewModel {
    pub allocation_id: Uuid,
    pub transaction_id: Uuid,
    pub actual_id: Uuid,
    pub amount: Money,
    pub tag: String,
}

impl AllocationViewModel {
    pub fn from_allocation(a: &TransactionAllocation) -> Self {
        Self {
            allocation_id: a.id,
            transaction_id: a.transaction_id,
            actual_id: a.actual_id,
            amount: a.amount,
            tag: a.tag.clone(),
        }
    }
}
