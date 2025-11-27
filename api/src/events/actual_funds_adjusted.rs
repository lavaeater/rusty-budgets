use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::events::ActualAdded;
use crate::models::Money;
use crate::models::{Budget, PeriodId};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualBudgetedFundsAdjusted {
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Money,
}

impl ActualBudgetedFundsAdjustedHandler for Budget {
    fn apply_adjust_actual_budgeted_funds(&mut self, event: &ActualBudgetedFundsAdjusted) -> Uuid {
        self.mutate_actual(event.period_id, event.actual_id, |actual| {
            actual.budgeted_amount += event.budgeted_amount;
        });
        event.actual_id
    }

    fn adjust_actual_budgeted_funds_impl(
        &self,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<ActualBudgetedFundsAdjusted, CommandError> {
        if let Some(period) = self.get_period(period_id) {
            if let Some(actual) = period.get_actual(actual_id) {
                if (actual.budgeted_amount + budgeted_amount) < Money::default() {
                    Err(CommandError::Validation(
                        "Items are not allowed to be less than zero.".to_string(),
                    ))
                } else {
                    Ok(ActualBudgetedFundsAdjusted {
                        budget_id: self.id,
                        actual_id,
                        period_id,
                        budgeted_amount,
                    })
                }
            } else {
                Err(CommandError::NotFound("Actual not found.".to_string()))
            }
        } else {
            Err(CommandError::NotFound("Period not found.".to_string()))
        }
    }
}
