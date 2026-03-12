use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct AllocationDeleted {
    pub budget_id: Uuid,
    #[event_id]
    pub allocation_id: Uuid,
    pub transaction_id: Uuid,
}

impl Display for AllocationDeleted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AllocationDeleted {{ budget_id: {}, allocation_id: {}, transaction_id: {} }}",
            self.budget_id, self.allocation_id, self.transaction_id
        )
    }
}

impl AllocationDeletedHandler for Budget {
    fn apply_delete_allocation(&mut self, event: &AllocationDeleted) -> Uuid {
        if let Some(period) = self.get_period_for_transaction_mut(event.transaction_id) {
            period.remove_allocation(event.allocation_id);
        }
        event.allocation_id
    }

    fn delete_allocation_impl(
        &self,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<AllocationDeleted, CommandError> {
        let period = self
            .get_period_for_transaction(transaction_id)
            .ok_or_else(|| {
                CommandError::NotFound(format!(
                    "Transaction {} not found in any period",
                    transaction_id
                ))
            })?;
        if !period.contains_allocation(allocation_id) {
            return Err(CommandError::NotFound(format!(
                "Allocation {} not found",
                allocation_id
            )));
        }
        Ok(AllocationDeleted {
            budget_id: self.id,
            allocation_id,
            transaction_id,
        })
    }
}
