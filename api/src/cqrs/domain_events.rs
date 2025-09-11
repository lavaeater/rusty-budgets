use crate::cqrs::budget::{Budget, BudgetGroup};
use crate::cqrs::framework::DomainEvent;
use crate::cqrs::framework::{Aggregate, CommandError};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        self.id = event.budget_id;
        self.name = event.name.clone();
        self.user_id = event.user_id;
        self.default_budget = event.default_budget;
    }
    
    fn create_budget_impl(&self, name: String, user_id: Uuid, default: bool) -> Result<BudgetCreated, CommandError> {
        if self.version == 0 && self.last_event == 0 {
            Ok(BudgetCreated {
                budget_id: self.id,
                name,
                user_id,
                default_budget: default,
            })
        } else {
            Err(CommandError::Validation("Budget already exists"))
        }
    }
    
    
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BudgetGroupAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
}

impl Budget {
    pub fn apply_add_budget_group(&mut self, event: &BudgetGroupAdded) {
        self.budget_groups.insert(event.budget_id, BudgetGroup::new(&event.name));
    }
    
    fn add_budget_group_impl(&self, group_id: Uuid, name: String) -> Result<BudgetGroupAdded, CommandError> {
        if self.budget_groups.contains_key(&group_id) || 
            self.budget_groups.values().any(|g| g.name == name) {
            Err(CommandError::Validation("Budget group already exists"))
        } else {
            Ok(BudgetGroupAdded {
                budget_id: self.id,
                group_id,
                name
            })
        }
    }
}