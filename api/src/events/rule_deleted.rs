use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct RuleDeleted {
    pub budget_id: Uuid,
    pub rule_id: Uuid,
}

impl RuleDeletedHandler for Budget {
    fn apply_delete_rule(&mut self, event: &RuleDeleted) -> Uuid {
        if let Some(rule) = self
            .match_rules
            .iter()
            .find(|r| r.id == event.rule_id)
            .cloned()
        {
            self.match_rules.remove(&rule);
        }
        event.rule_id
    }

    fn delete_rule_impl(&self, rule_id: Uuid) -> Result<RuleDeleted, CommandError> {
        if !self.match_rules.iter().any(|r| r.id == rule_id) {
            return Err(CommandError::NotFound("Rule not found".to_string()));
        }
        Ok(RuleDeleted {
            budget_id: self.id,
            rule_id,
        })
    }
}
