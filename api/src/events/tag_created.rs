use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, Periodicity, Tag};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TagCreated {
    pub budget_id: Uuid,
    #[event_id]
    pub tag_id: Uuid,
    pub name: String,
    pub periodicity: Periodicity,
}

impl TagCreatedHandler for Budget {
    fn apply_create_tag(&mut self, event: &TagCreated) -> Uuid {
        self.tags.push(Tag::new(event.tag_id, event.name.clone(), event.periodicity));
        event.tag_id
    }

    fn create_tag_impl(&self, name: String, periodicity: Periodicity) -> Result<TagCreated, CommandError> {
        if self.contains_tag_with_name(&name) {
            return Err(CommandError::Validation("Tag already exists.".to_string()));
        }
        Ok(TagCreated {
            budget_id: self.id,
            tag_id: Uuid::new_v4(),
            name,
            periodicity,
        })
    }
}
