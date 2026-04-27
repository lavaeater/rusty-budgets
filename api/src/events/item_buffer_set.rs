use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, Money};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "set_item_buffer")]
pub struct ItemBufferSet {
    pub budget_id: Uuid,
    pub item_id: Uuid,
    /// None clears the buffer target.
    pub buffer_target: Option<Money>,
}

impl ItemBufferSetHandler for Budget {
    fn apply_set_item_buffer(&mut self, event: &ItemBufferSet) -> Uuid {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == event.item_id) {
            item.buffer_target = event.buffer_target;
        }
        event.item_id
    }

    fn set_item_buffer_impl(
        &self,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<ItemBufferSet, CommandError> {
        if !self.contains_budget_item(item_id) {
            return Err(CommandError::NotFound("Budget item not found".to_string()));
        }
        Ok(ItemBufferSet {
            budget_id: self.id,
            item_id,
            buffer_target,
        })
    }
}
