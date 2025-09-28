use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::Money;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemFundsAdjusted {
    budget_id: Uuid,
    item_id: Uuid,
    amount: Money,
}

impl ItemFundsAdjustedHandler for Budget {
    fn apply_adjust_item_funds(&mut self, event: &ItemFundsAdjusted) -> Uuid {
        self.budget_items.add_budgeted_amount(&event.item_id, event.amount);
        let item_type = self.budget_items.type_for(&event.item_id).unwrap();
        self.budgeted_by_type.entry(*item_type)
            .and_modify(|v|
                {*v += event.amount})
            .or_insert(event.amount);
        self.recalc_overview();
        event.item_id
    }

    fn adjust_item_funds_impl(
        &self,
        item_id: Uuid,
        amount: Money,
    ) -> Result<ItemFundsAdjusted, CommandError> {
        let item = self.get_item(&item_id);

        if item.is_none() {
            return Err(CommandError::Validation("Item does not exist"));
        }
        let item = item.unwrap();

        if (item.budgeted_amount + amount) < Money::default() {
            return Err(CommandError::Validation(
                "Items are not allowed to be less than zero.",
            ));
        }

        Ok(ItemFundsAdjusted {
            budget_id: self.id,
            item_id,
            amount,
        })
    }
}