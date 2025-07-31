use chrono::NaiveDate;
use joydb::{JoydbError, Model};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::Db;
use crate::models::budget::Budget;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct User {
    pub id: Uuid,
    pub user_name: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub birthday: Option<NaiveDate>,
}

impl User {
    pub fn new(user_name: &str, email: &str, first_name: &str, last_name: &str, phone: Option<String>, birthday: Option<NaiveDate>) -> Self {
        Self {
            id: Uuid::new_v4(),
            user_name: user_name.to_string(),
            email: email.to_string(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            phone,
            birthday,
        }
    }
    
    pub fn get_default_budget(&self, db: &Db) -> anyhow::Result<Budget> {
        match db.get_all_by(|b: &Budget| b.user_id == self.id && b.default_budget) {
            Ok(mut budgets) => { Ok(budgets.remove(0)) },
            Err(e) => { Err(anyhow::Error::from(e)) }
        }
    }
}
