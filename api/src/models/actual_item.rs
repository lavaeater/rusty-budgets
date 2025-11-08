use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{BudgetItem, PeriodId, Money, BudgetingType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualView {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub period_id: PeriodId,
    pub name: String,
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

impl ActualView {
    pub fn new(actual: &ActualItem, budget_item: &BudgetItem) -> ActualView {
        ActualView {
            id: actual.id,
            budget_item_id: actual.budget_item_id,
            period_id: actual.period_id,
            name: budget_item.name.clone(),
            budgeting_type: budget_item.budgeting_type,
            budgeted_amount: actual.budgeted_amount,
            actual_amount: actual.actual_amount,
            notes: actual.notes.clone(),
            tags: actual.tags.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualItem {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub period_id: PeriodId,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

impl ActualItem {
    pub fn new(
        id: Uuid,
        budget_item_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
        actual_amount: Money,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> ActualItem {
        ActualItem {
            id,
            budget_item_id,
            period_id,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        }
    }
}
