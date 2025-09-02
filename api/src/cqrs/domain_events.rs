use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::budgets::Budget;
use crate::cqrs::framework::{Aggregate, CommandError};
use crate::cqrs::framework::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "create_budget", command_error = "CommandError")]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
}

impl  for BudgetCreated {}

impl DomainEvent<Budget> for BudgetCreated {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        todo!()
    }
}

