use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use crate::cqrs::budgets::Budget;
use crate::cqrs::framework::{Aggregate, CommandError};
use crate::cqrs::framework::DomainEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "create_budget")]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
}

impl BudgetCreated {
    pub fn new(budget_id: Uuid, name: String, user_id: Uuid, default: bool) -> Self {
        Self {
            budget_id,
            name,
            user_id,
            default,
        }
    }
}

impl DomainEvent<Budget> for BudgetCreated
{
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id
    { self.budget_id }
    fn apply(&self, state: &mut Budget) {
        // Initialize the budget with data from the BudgetCreated event
        state.id = self.budget_id;
        state.name = self.name.clone();
        state.user_id = self.user_id;
        state.default_budget = self.default;
        // Other fields (budget_groups, bank_transactions) remain as initialized (empty)
    }
}
impl Budget
{
    pub fn create_budget(&mut self, args: impl Into<BudgetCreated>) ->
    Result<BudgetCreated, CommandError>
    {
        let event = args.into();
        // Apply the event to update this aggregate's state
        event.apply(self);
        Ok(event)
    }
}
