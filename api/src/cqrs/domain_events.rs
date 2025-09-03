use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::budget::Budget;
use crate::cqrs::budgets::BudgetEvent;
use crate::cqrs::framework::{Aggregate, CommandError, Decision};
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

impl Decision<Budget, BudgetEvent> for BudgetCreated {
    fn decide(self, state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
        match state {
            None => Ok(BudgetEvent::BudgetCreated(self)),
            Some(_) => Err(CommandError::Validation("Budget already exists")),
        }
    }
}

impl DomainEvent<Budget> for BudgetCreated {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.name = self.name.clone();
        state.default_budget = self.default;
    }
}

