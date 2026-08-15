use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, Periodicity, Tag};
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
                tag.name.clone_from(name);
            }
            // Legacy shape: a periodicity change re-derives the classification,
            // unless the user has answered explicitly via `TagClassified` — a
            // deliberate choice must not be silently overwritten by an edit
            // through the older periodicity-only path.
            if let Some(periodicity) = event.periodicity
                && !tag.explicitly_classified
            {
                let derived =
                    Tag::from_legacy_periodicity(tag.id, tag.name.clone(), periodicity);
                tag.cost_kind = derived.cost_kind;
                tag.matching = derived.matching;
                tag.needs_review = derived.needs_review;
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
