use serde::{Deserialize, Serialize};
use uuid::Uuid;
use joydb::Model;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: String,
    pub expected_at: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl BudgetItem {
    pub fn new_from_user(budget_id: Uuid, name: &str, item_type: &str, expected_at: chrono::NaiveDate, created_by: Uuid) -> BudgetItem {
        BudgetItem {
            id: Uuid::new_v4(),
            budget_id,
            name: name.to_string(),
            item_type: item_type.to_string(),
            expected_at,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}
