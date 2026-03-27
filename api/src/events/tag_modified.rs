use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, Periodicity};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TagModified {
    pub budget_id: Uuid,
    pub tag_id: Uuid,
    pub name: Option<String>,
    pub periodicity: Option<Periodicity>,
    pub deleted: Option<bool>,
}

impl TagModifiedHandler for Budget {
    fn apply_modify_tag(&mut self, event: &TagModified) -> Uuid {
        if let Some(tag) = self.tags.iter_mut().find(|t| t.id == event.tag_id) {
            if let Some(name) = &event.name {
                tag.name = name.clone();
            }
            if let Some(periodicity) = event.periodicity {
                tag.periodicity = periodicity;
            }
            if let Some(deleted) = event.deleted {
                tag.deleted = deleted;
            }
        }
        event.tag_id
    }

    fn modify_tag_impl(
        &self,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<TagModified, CommandError> {
        if !self.contains_tag(tag_id) {
            return Err(CommandError::NotFound("Tag not found".to_string()));
        }
        Ok(TagModified {
            budget_id: self.id,
            tag_id,
            name,
            periodicity,
            deleted,
        })
    }
}
