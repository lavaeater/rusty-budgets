use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, PeriodId};
use crate::models::BudgetingType;
use crate::models::Money;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualFundsReallocated {
    budget_id: Uuid,
    period_id: PeriodId,
    from_actual_id: Uuid,
    to_actual_id: Uuid,
    amount: Money,
}

impl ActualFundsReallocatedHandler for Budget {
    fn apply_reallocate_actual_funds(&mut self, event: &ActualFundsReallocated) -> Uuid {
        self.with_period_mut(event.period_id).mutate_actual(event.from_actual_id, |actual| {
            actual.actual_amount -= event.amount;
        });
        self.with_period_mut(event.period_id).mutate_actual(event.to_actual_id, |actual| {
            actual.actual_amount += event.amount;
        });
        
        event.from_actual_id
    }

    fn reallocate_actual_funds_impl(
        &self,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<ActualFundsReallocated, CommandError> {
        /*
        Re-allocations of funds are only allowed if both items are of
        budget item type expense OR savings - income cannot be reallocated, only modified.
         */
        if let Some(period) = self.get_period(period_id) {
            let from_item = period.get_actual(from_actual_id);
            let to_item = period.get_actual(to_actual_id);
            if from_item.is_none() || to_item.is_none() {
                return Err(CommandError::Validation(
                    "Either Actual Item to take funds from or Actual Item to deliver funds to does not exist.".to_string(),
                ));
            }
            
            let from_item = from_item.unwrap();
            let to_item = to_item.unwrap();
            
            let from_type = from_item.budgeting_type();
            let to_type = to_item.budgeting_type();
            
            if from_type == BudgetingType::Income
                || to_type == BudgetingType::Income
            {
                return Err(CommandError::Validation("Re-allocations of funds are only allowed if both items are of budget item type expense OR savings - income cannot be reallocated, only modified.".to_string()));
            }
            
            if from_item.budgeted_amount < amount {
                return Err(CommandError::Validation(
                    "Item to take funds from does not have enough budgeted amount.".to_string(),
                ));
            }
            Ok(ActualFundsReallocated {
                budget_id: self.id,
                period_id,
                from_actual_id,
                to_actual_id,
                amount,
            })
        } else {
            Err(CommandError::Validation(format!("Period does not exist: {}", period_id)))
        }
    }
}