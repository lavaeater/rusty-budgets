use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::MatchRule;
use core::fmt::{Display, Formatter};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TransactionConnected,
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct RuleAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub rule_id: Uuid,
    pub transaction_key: Vec<String>,
    pub item_key: Vec<String>,
    pub always_apply: bool,
    pub tag_id: Option<Uuid>,
}

impl Display for RuleAdded {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "RuleAdded {{ budget_id: {}, transaction_key: {:?}, item_key: {:?}, always_apply: {} }}",
            self.budget_id, self.transaction_key, self.item_key, self.always_apply
        )
    }
}

impl RuleAddedHandler for Budget {
    fn apply_add_rule(&mut self, event: &RuleAdded) -> Uuid {
        self.match_rules.insert(MatchRule {
            id: event.rule_id,
            transaction_key: event.transaction_key.clone(),
            item_key: event.item_key.clone(),
            always_apply: event.always_apply,
            tag_id: event.tag_id,
        });

        event.rule_id
    }

    fn add_rule_impl(
        &self,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<RuleAdded, CommandError> {
        let rule = MatchRule {
            id: Uuid::new_v4(),
            transaction_key: transaction_key.clone(),
            item_key: item_key.clone(),
            always_apply,
            tag_id,
        };
        if self.match_rules.contains(&rule) {
            return Err(CommandError::Validation("Rule already exists.".to_string()));
        }
        Ok(RuleAdded {
            budget_id: self.id,
            rule_id: rule.id,
            transaction_key,
            item_key,
            always_apply,
            tag_id,
        })
    }
}
