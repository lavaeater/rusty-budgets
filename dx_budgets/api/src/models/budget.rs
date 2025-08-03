use serde::{Deserialize, Serialize};
use uuid::Uuid;
use joydb::Model;
use crate::models::budget_item::BudgetItem;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub budget_items: Vec<BudgetItem>,
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
            ..Default::default()
        }
    }
    
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}
