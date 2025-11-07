use uuid::Uuid;
use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::Currency;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BudgetCreated {
    #[event_id]
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default_budget: bool,
    pub currency: Currency,
}

impl BudgetCreatedHandler for Budget {
    fn apply_create_budget(&mut self, event: &BudgetCreated) -> Uuid {
        self.id = event.budget_id;
        self.name = event.name.clone();
        self.user_id = event.user_id;
        self.default_budget = event.default_budget;
        self.currency = event.currency;
        self.id
    }

    fn create_budget_impl(
        &self,
        name: String,
        user_id: Uuid,
        default_budget: bool,
        currency: Currency,
    ) -> Result<BudgetCreated, CommandError> {
        if self.version == 0 && self.last_event == 0 {
            Ok(BudgetCreated {
                budget_id: Uuid::new_v4(),
                name,
                user_id,
                default_budget,
                currency,
            })
        } else {
            Err(CommandError::Validation("Budget already exists".to_string()))
        }
    }
}