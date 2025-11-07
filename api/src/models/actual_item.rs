use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{BudgetItem, PeriodId, Money};

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
