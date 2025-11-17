use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::events::ActualAdded;
use crate::models::Money;
use crate::models::{Budget, PeriodId};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualFundsAdjusted {
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Money,
}

impl ActualFundsAdjustedHandler for Budget {
    fn apply_adjust_actual_funds(&mut self, event: &ActualFundsAdjusted) -> Uuid {
        self.mutate_actual(event.period_id, event.actual_id, |actual| {
            actual.actual_amount += event.budgeted_amount;
        });
        event.actual_id
    }

    fn adjust_actual_funds_impl(
        &self,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<ActualFundsAdjusted, CommandError> {
        if let Some(period) = self.get_period(period_id) {
            if let Some(actual) = period.get_actual(actual_id) {
                if (actual.budgeted_amount + budgeted_amount) < Money::default() {
                    Err(CommandError::Validation(
                        "Items are not allowed to be less than zero.".to_string(),
                    ))
                } else {
                    Ok(ActualFundsAdjusted {
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
