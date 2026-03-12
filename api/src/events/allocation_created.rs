use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, Money, TransactionAllocation};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct AllocationCreated {
    pub budget_id: Uuid,
    #[event_id]
    pub allocation_id: Uuid,
    pub transaction_id: Uuid,
    pub actual_id: Uuid,
    pub amount: Money,
    pub tag: String,
}

impl Display for AllocationCreated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AllocationCreated {{ budget_id: {}, allocation_id: {}, transaction_id: {}, actual_id: {}, tag: {} }}",
            self.budget_id, self.allocation_id, self.transaction_id, self.actual_id, self.tag
        )
    }
}

impl AllocationCreatedHandler for Budget {
    fn apply_create_allocation(&mut self, event: &AllocationCreated) -> Uuid {
        let allocation = TransactionAllocation {
            id: event.allocation_id,
            transaction_id: event.transaction_id,
            actual_id: event.actual_id,
            amount: event.amount,
            tag: event.tag.clone(),
        };
        if let Some(period) = self.get_period_for_transaction_mut(event.transaction_id) {
            period.add_allocation(allocation);
        }
        event.allocation_id
    }

    fn create_allocation_impl(
        &self,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<AllocationCreated, CommandError> {
        if !self.contains_transaction(transaction_id) {
            return Err(CommandError::NotFound(format!(
                "Transaction {} not found",
                transaction_id
            )));
        }
        let period = self
            .get_period_for_transaction(transaction_id)
            .ok_or_else(|| CommandError::NotFound("Period not found for transaction".to_string()))?;
        if !period.contains_actual(actual_id) {
            return Err(CommandError::NotFound(format!(
                "ActualItem {} not found in period",
                actual_id
            )));
        }
        Ok(AllocationCreated {
            budget_id: self.id,
            allocation_id: Uuid::new_v4(),
            transaction_id,
            actual_id,
            amount,
            tag,
        })
    }
}
