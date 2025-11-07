use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetPeriodId};
use crate::models::Money;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualFundsAdjusted {
    budget_id: Uuid,
    actual_id: Uuid,
    budget_period_id: BudgetPeriodId,
    amount: Money,
}

impl ActualFundsAdjustedHandler for Budget {
    fn apply_adjust_actual_funds(&mut self, event: &ActualFundsAdjusted) -> Uuid {
        self.with_period_mut(event.budget_period_id).add_actual_amount_to_item(&event.actual_id, &event.amount);
        let item_type = self.type_for_item(&event.item_id).unwrap();
        self.update_budget_budgeted_amount(event.budget_period_id, &item_type, &event.amount);
        self.recalc_overview(event.budget_period_id);
        event.item_id
    }

    fn adjust_actual_funds_impl(
        &self,
        actual_id: Uuid,
        budget_period_id: BudgetPeriodId,
        amount: Money,
    ) -> Result<ActualFundsAdjusted, CommandError> {
        let item = self.with_period_or_now(budget_period_id).budget_items.get(&item_id);

        if item.is_none() {
            return Err(CommandError::Validation("Item does not exist"));
        }
        let item = item.unwrap();

        if (item.budgeted_amount + amount) < Money::default() {
            return Err(CommandError::Validation(
                "Items are not allowed to be less than zero.",
            ));
        }

        Ok(ActualFundsAdjusted {
            budget_id: self.id,
            item_id,
            budget_period_id,
            amount,
        })
    }
}