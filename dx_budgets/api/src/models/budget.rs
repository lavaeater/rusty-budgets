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
