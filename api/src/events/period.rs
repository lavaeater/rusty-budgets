use serde::{Serialize, Deserialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::Money;

/// Events that occur within a specific Budget Period (e.g. "2025-09")
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetPeriodEvent {
    TransactionImportedToPeriod {
        budget_id: Uuid,
        period_id: String, // "YYYY-MM"
        transaction_id: Uuid,
        date: DateTime<Utc>,
        description: String,
        amount: Money,
    },
    TransactionSuggestedAssignment {
        budget_id: Uuid,
        period_id: String,
        transaction_id: Uuid,
        suggested_item_id: Option<Uuid>,
    },
    BudgetItemAddedToPeriod {
        budget_id: Uuid,
        period_id: String,
        item_id: Uuid,
        name: String,
        planned_amount: Money,
    },
    BudgetItemUpdatedInPeriod {
        budget_id: Uuid,
        period_id: String,
        item_id: Uuid,
        new_name: Option<String>,
        new_planned_amount: Option<Money>,
    },
}
