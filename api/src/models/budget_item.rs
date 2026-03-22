use crate::models::budgeting_type::BudgetingType;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Periodicity {
    #[default]
    Monthly,
    Quarterly,
    Annual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budgeting_type: BudgetingType,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub periodicity: Periodicity,
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
            tags: Vec::new(),
            periodicity: Periodicity::default(),
        }
    }
}
