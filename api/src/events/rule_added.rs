use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use core::fmt::{Display, Formatter};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::MatchRule;

// TransactionConnected,
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct RuleAdded {
    pub budget_id: Uuid,
    pub transaction_key: Vec<String>,
    pub item_name: String,
    pub always_apply: bool
}

impl Display for RuleAdded {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "RuleAdded {{ budget_id: {}, transaction_key: {:?}, item_name: {}, always_apply: {} }}", self.budget_id, self.transaction_key, self.item_name, self.always_apply)
    }
}

impl RuleAddedHandler for Budget {
    fn apply_add_rule(&mut self, event: &RuleAdded) -> Uuid {
        self.match_rules.insert(MatchRule {
            transaction_key: event.transaction_key.clone(),
            item_name: event.item_name.clone(),
            always_apply: event.always_apply
        });
        
        event.budget_id
    }

    fn add_rule_impl(&self, transaction_key: Vec<String>, item_name: String, always_apply: bool) -> Result<RuleAdded, CommandError> {
        let rule = MatchRule {
            transaction_key: transaction_key.clone(),
            item_name: item_name.clone(),
            always_apply
        };
        if self.match_rules.contains(&rule) {
            return Err(CommandError::Validation("Rule already exists."));
        }
        Ok(RuleAdded {
            budget_id: self.id,
            transaction_key,
            item_name,
            always_apply
        })
    }
}
