use serde::{Deserialize, Serialize};
use uuid::Uuid;
use cqrs_macros::DomainEvent;
use crate::cqrs::budget::Budget;
use crate::cqrs::framework::{Aggregate, CommandError, Decision};
use crate::cqrs::framework::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default_budget: bool,
}

impl Budget {
    pub fn apply_create_budget(&mut self, event: &BudgetCreated) {
        
    }
}
