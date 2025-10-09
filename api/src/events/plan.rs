use serde::{Serialize, Deserialize};
use uuid::Uuid;
use crate::models::Money;

/// Events that modify the long-lived Budget Plan structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetPlanEvent {
    BudgetItemAddedToPlan {
        budget_id: Uuid,
        item_id: Uuid,
        name: String,
        planned_amount: Money,
    },
    BudgetItemUpdatedInPlan {
        budget_id: Uuid,
        item_id: Uuid,
        new_name: Option<String>,
        new_planned_amount: Option<Money>,
    },
    BudgetItemArchivedInPlan {
        budget_id: Uuid,
        item_id: Uuid,
    },
}
