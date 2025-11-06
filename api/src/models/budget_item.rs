use crate::models::budgeting_type::BudgetingType;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budgeting_type: BudgetingType,
}

impl BudgetItem {
    pub fn new(
        id: Uuid,
        name: &str,
        budgeting_type: BudgetingType,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            budgeting_type,
        }
    }
}
