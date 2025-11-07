use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::BudgetingType;
use crate::models::Money;
use cqrs_macros::DomainEvent;
use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemModified {
    pub budget_id: Uuid,
    pub item_id: Uuid,
    pub name: Option<String>,
    pub item_type: Option<BudgetingType>,
}

impl ItemModifiedHandler for Budget {
    fn apply_modify_item(&mut self, event: &ItemModified) -> Uuid {
        self.modify_budget_item(
            &event.item_id,
            event.name.clone(),
            event.item_type
        );
        event.item_id
    }

    fn modify_item_impl(
        &self,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
    ) -> Result<ItemModified, CommandError> {
        if self.contains_budget_item(&item_id) {
            Ok(ItemModified {
                budget_id: self.id,
                item_id,
                name,
                item_type
            })
        } else {
            tracing::error!("Budget Item not found");
            Err(CommandError::NotFound("Budget Item not found".to_string()))
        }
    }
}
