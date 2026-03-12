use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetingType, PeriodId};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "delete_allocation")]
pub struct AllocationDeleted {
    pub budget_id: Uuid,
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
        let cost_types = [BudgetingType::Expense, BudgetingType::Savings];
        if let Some(tx) = self.get_transaction(event.transaction_id) {
            let period_id = PeriodId::from_date(tx.date, self.month_begins_on());
            let period = self.with_period_mut(period_id);
            let amount = period
                .allocations
                .iter()
                .find(|a| a.id == event.allocation_id)
                .map(|a| (a.amount, a.actual_id));
            period.remove_allocation(event.allocation_id);
            if let Some((amount, actual_id)) = amount {
                period.mutate_actual(actual_id, |a| {
                    let signed = if cost_types.contains(&a.budgeting_type) { -amount } else { amount };
                    a.actual_amount -= signed;
                });
            }
        }
        event.budget_id
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
