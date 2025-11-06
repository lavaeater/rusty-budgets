use crate::models::budgeting_type::BudgetingType;
use crate::models::money::Money;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActualBudgetItem {
    pub id: Uuid,
    pub budget_item: Arc<Mutex<BudgetItem>>,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

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
