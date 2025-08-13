use serde::{Deserialize, Serialize};
use joydb::Model;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BankTransaction {
    pub id: Uuid,
    pub text: String,
    pub amount: f32,
    pub budget_item: Uuid,
    pub bank_date: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl Eq for BankTransaction {}

impl BankTransaction {
    pub fn new_from_user(
        text: &str,
        amount: f32,
        budget_item: Uuid,
        bank_date: chrono::NaiveDate,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            amount,
            budget_item,
            bank_date,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}