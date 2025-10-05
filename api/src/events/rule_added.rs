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
    budget_id: Uuid,
    rule: MatchRule,
}

impl Display for RuleAdded {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "RuleAdded {{ budget_id: {}, rule: {} }}", self.budget_id, self.rule)
    }
}


impl RuleAddedHandler for Budget {
    fn apply_add_rule(&mut self, event: &RuleAdded) -> Uuid {
        println!("Applying rule added event: {}", event);

        // First, extract all the data we need from the transaction (immutable borrow)
    }

    fn add_rule_impl(&self, rule: MatchRule) -> Result<RuleAdded, CommandError> {
    }
}
