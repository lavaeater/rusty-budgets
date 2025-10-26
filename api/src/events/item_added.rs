use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::BudgetItem;
use crate::models::BudgetingType;
use crate::models::Money;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub item_id: Uuid,
    pub name: String,
    pub item_type: BudgetingType,
    pub budgeted_amount: Money,
}

impl ItemAddedHandler for Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) -> Uuid {
        let new_item = BudgetItem::new(
            event.item_id,
            &event.name,
            event.budgeted_amount,
            None,
            None,
        );
        let new_item_id = new_item.id;
        self.insert_item(&new_item, event.item_type);
        new_item_id
    }

    fn add_item_impl(
        &self,
        name: String,
        item_type: BudgetingType,
        budgeted_amount: Money,
    ) -> Result<ItemAdded, CommandError> {
        if self.contains_item_with_name(&name) {
            return Err(CommandError::Validation("Item already exists."));
        }
        Ok(ItemAdded {
            budget_id: self.id,
            item_id: Uuid::new_v4(),
            name,
            item_type,
            budgeted_amount,
        })
    }
}
