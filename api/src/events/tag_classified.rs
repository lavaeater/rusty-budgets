use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, CostKind, Matching};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Records a deliberate answer to "is this a bill, and can we auto-apply it?".
///
/// Deliberately a *new* event rather than a change to `TagCreated`: the log
/// holds thousands of historical `TagCreated`s carrying only a `Periodicity`,
/// and rewriting history is not an option in an event-sourced aggregate. Replay
/// derives a provisional classification from the legacy field
/// ([`crate::models::Tag::from_legacy_periodicity`]); this event overrides it
/// and clears the review flag.
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TagClassified {
    pub budget_id: Uuid,
    pub tag_id: Uuid,
    pub cost_kind: CostKind,
    pub matching: Matching,
}

impl TagClassifiedHandler for Budget {
    fn apply_classify_tag(&mut self, event: &TagClassified) -> Uuid {
        if let Some(tag) = self.tags.iter_mut().find(|t| t.id == event.tag_id) {
            tag.cost_kind = event.cost_kind;
            tag.matching = event.matching;
            tag.needs_review = false;
            tag.explicitly_classified = true;
        }
        event.tag_id
    }

    fn classify_tag_impl(
        &self,
        tag_id: Uuid,
        cost_kind: CostKind,
        matching: Matching,
    ) -> Result<TagClassified, CommandError> {
        if !self.contains_tag(tag_id) {
            return Err(CommandError::NotFound("Tag not found".to_string()));
        }
        Ok(TagClassified {
            budget_id: self.id,
            tag_id,
            cost_kind,
            matching,
        })
    }
}
