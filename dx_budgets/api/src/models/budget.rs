use crate::User;
use crate::models::budget_item::BudgetItem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use joydb::{Joydb, adapters::JsonAdapter, Model};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub user_id: Uuid,
}

impl Budget {
    pub fn new(name: &str, default_budget: bool, user_id: Uuid) -> Budget {
        Budget {
            id: Uuid::new_v4(),
            name: name.to_string(),
            default_budget,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            user_id,
        }
    }
    
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

pub fn before_create(budget: &mut Budget) -> welds::errors::Result<()> {
    budget.id = Uuid::new_v4();
    budget.created_at = chrono::Utc::now().naive_utc();
    budget.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

pub fn before_update(budget: &mut Budget) -> welds::errors::Result<()> {
    budget.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}
