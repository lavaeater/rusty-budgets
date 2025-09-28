use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::BudgetingType;
use crate::models::Money;

// FundsReallocated
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemFundsReallocated {
    budget_id: Uuid,
    from_item_id: Uuid,
    to_item_id: Uuid,
    amount: Money,
}

impl ItemFundsReallocatedHandler for Budget {
    fn apply_reallocate_item_funds(&mut self, event: &ItemFundsReallocated) -> Uuid {
        self.budget_items.add_budgeted_amount(&event.from_item_id, -event.amount);
        
        let from_type = self.budget_items.type_for(&event.from_item_id).unwrap();
        self.budgeted_by_type
            .entry(*from_type)
            .and_modify(|v| {
                *v -= event.amount;
            }).or_insert(-event.amount);

        
        self.budget_items.add_budgeted_amount(&event.to_item_id, event.amount);
        
        let to_type = self.budget_items.type_for(&event.to_item_id).unwrap();
        self.budgeted_by_type
            .entry(*to_type)
            .and_modify(|v| {
                *v += event.amount;
            }).or_insert(event.amount);
        self.recalc_overview();
        event.from_item_id
    }

    fn reallocate_item_funds_impl(
        &self,
        from_item_id: Uuid,
        to_item_id: Uuid,
        amount: Money,
    ) -> Result<ItemFundsReallocated, CommandError> {
        /*
        Re-allocations of funds are only allowed if both items are of
        budget item type expense OR savings - income cannot be reallocated, only modified.
         */
        let from_item = self.budget_items.get(&from_item_id);
        let to_item = self.budget_items.get(&to_item_id);

        if from_item.is_none() || to_item.is_none() {
            return Err(CommandError::Validation(
                "Either Item to take funds from or Item to deliver funds to does not exist.",
            ));
        }
        let from_type = self.budget_items.type_for(&from_item_id).unwrap();
        let to_type = self.budget_items.type_for(&to_item_id).unwrap();

        if from_type == &BudgetingType::Income
            || to_type == &BudgetingType::Income
        {
            return Err(CommandError::Validation("Re-allocations of funds are only allowed if both items are of budget item type expense OR savings - income cannot be reallocated, only modified."));
        }

        let from_item = from_item.unwrap();

        if from_item.budgeted_amount < amount {
            return Err(CommandError::Validation(
                "Item to take funds from does not have enough budgeted amount.",
            ));
        }

        Ok(ItemFundsReallocated {
            budget_id: self.id,
            from_item_id,
            to_item_id,
            amount,
        })
    }
}