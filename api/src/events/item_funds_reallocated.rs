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
        self.add_budgeted_amount_to_item(&event.from_item_id, &-event.amount);
        
        let from_type = self.type_for_item(&event.from_item_id).unwrap();
        self.update_budget_budgeted_amount(None, &from_type, &-event.amount);

        
        self.add_budgeted_amount_to_item(&event.to_item_id, &event.amount);
        
        let to_type = self.type_for_item(&event.to_item_id).unwrap();
        self.update_budget_budgeted_amount(None, &to_type, &event.amount);
        
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
        let from_item = self.get_item(&from_item_id);
        let to_item = self.get_item(&to_item_id);

        if from_item.is_none() || to_item.is_none() {
            return Err(CommandError::Validation(
                "Either Item to take funds from or Item to deliver funds to does not exist.",
            ));
        }
        let from_type = self.type_for_item(&from_item_id).unwrap();
        let to_type = self.type_for_item(&to_item_id).unwrap();

        if from_type == BudgetingType::Income
            || to_type == BudgetingType::Income
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