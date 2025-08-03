use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use joydb::Model;
use crate::models::budget_item::BudgetItem;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum MonthBeginsOn {
    #[default]
    PreviousMonth(u32),
    CurrentMonth(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub month_begins_on: MonthBeginsOn,
    pub budget_items: HashMap<Uuid, BudgetItem>,
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
            month_begins_on: MonthBeginsOn::PreviousMonth(25),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            user_id,
            ..Default::default()
        }
    }
    
    pub fn store_budget_item(&mut self, budget_item: &BudgetItem) {
        match self.budget_items.entry(budget_item.id) {
            Vacant(e) => {
                e.insert(budget_item.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_item.clone());
            }
        }
        self.touch();
    }
    
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}
