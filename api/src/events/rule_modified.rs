use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct RuleModified {
    pub budget_id: Uuid,
    pub rule_id: Uuid,
    pub transaction_key: Vec<String>,
}

impl RuleModifiedHandler for Budget {
    fn apply_modify_rule(&mut self, event: &RuleModified) -> Uuid {
        if let Some(old_rule) = self.match_rules.iter().find(|r| r.id == event.rule_id).cloned() {
            self.match_rules.remove(&old_rule);
            let mut updated = old_rule;
            updated.transaction_key = event.transaction_key.clone();
            self.match_rules.insert(updated);
        }
        event.rule_id
    }

    fn modify_rule_impl(
        &self,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<RuleModified, CommandError> {
        if !self.match_rules.iter().any(|r| r.id == rule_id) {
            return Err(CommandError::NotFound("Rule not found".to_string()));
        }
        Ok(RuleModified {
            budget_id: self.id,
            rule_id,
            transaction_key,
        })
    }
}
