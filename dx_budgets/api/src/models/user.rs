use chrono::NaiveDate;
use joydb::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}
