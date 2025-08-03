use joydb::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::budget_transaction::BudgetTransaction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemType {
    Income,
    #[default]
    Expense,
    Savings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemPeriodicity {
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budget_item_type: BudgetItemType,
    pub periodicity: BudgetItemPeriodicity,
    pub starts_date: chrono::NaiveDate,
    pub incoming_transactions: Vec<BudgetTransaction>,
    pub outgoing_transactions: Vec<BudgetTransaction>,
    pub bank_transactions: Vec<BudgetTransaction>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl BudgetItem {
    pub fn new_from_user(
        budget_id: Uuid,
        name: &str,
        budget_item_type: BudgetItemType,
        periodicity: BudgetItemPeriodicity,
        starts_date: chrono::NaiveDate,
        created_by: &Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            name: name.to_string(),
            budget_item_type,
            periodicity,
            starts_date,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by: created_by.clone(),
            ..Default::default()
        }
    }
}
