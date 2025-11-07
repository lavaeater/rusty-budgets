use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetItem, BudgetingType};
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
}

impl ItemAddedHandler for Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) -> Uuid {
        let new_item = BudgetItem::new(event.item_id, &event.name, event.item_type);

        self.budget_items.insert(event.item_id, new_item);

        event.item_id
    }

    fn add_item_impl(
        &self,
        name: String,
        item_type: BudgetingType,
    ) -> Result<ItemAdded, CommandError> {
        if self.contains_item_with_name(&name) {
            return Err(CommandError::Validation("Item already exists.".to_string()));
        }
        Ok(ItemAdded {
            budget_id: self.id,
            item_id: Uuid::new_v4(),
            name,
            item_type,
        })
    }
}
