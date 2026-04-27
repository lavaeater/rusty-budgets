use crate::models::budgeting_type::BudgetingType;
use crate::models::money::Money;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Periodicity {
    Monthly,
    Quarterly,
    Annual,
    #[default]
    OneOff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budgeting_type: BudgetingType,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
    #[serde(default)]
    pub periodicity: Periodicity,
    #[serde(default)]
    pub buffer_target: Option<Money>,
}

impl BudgetItem {
    pub fn new(id: Uuid, name: &str, budgeting_type: BudgetingType) -> Self {
        Self {
            id,
            name: name.to_string(),
            budgeting_type,
            tag_ids: Vec::new(),
            periodicity: Periodicity::default(),
            buffer_target: None,
        }
    }
}
