use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use welds::prelude::*;

/// Append-only event log. `data` holds the serialised `BudgetEvent` JSON.
#[derive(Debug, Clone, WeldsModel, Serialize, Deserialize)]
#[welds(table = "budget_events")]
pub struct PgStoredBudgetEvent {
    #[welds(primary_key)]
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub timestamp: i64,
    pub created_at: DateTime<Utc>,
    pub user_id: Uuid,
    pub data: JsonValue,
}

/// Snapshot of a `Budget` aggregate. `data` holds the full JSON snapshot.
#[derive(Debug, Clone, WeldsModel, Serialize, Deserialize)]
#[welds(table = "budgets")]
pub struct PgBudget {
    #[welds(primary_key)]
    pub id: Uuid,
    pub version: i64,
    pub last_event: i64,
    pub data: JsonValue,
}

/// Application user.
#[derive(Debug, Clone, WeldsModel, Serialize, Deserialize)]
#[welds(table = "users")]
pub struct PgUser {
    #[welds(primary_key)]
    pub id: Uuid,
    pub user_name: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: Option<String>,
    pub birthday: Option<NaiveDate>,
}

/// Maps a user to their list of budget IDs + default flag.
/// `budgets` stores `Vec<(Uuid, bool)>` as JSONB.
#[derive(Debug, Clone, WeldsModel, Serialize, Deserialize)]
#[welds(table = "user_budgets")]
pub struct PgUserBudgets {
    #[welds(primary_key)]
    pub id: Uuid,
    pub budgets: JsonValue,
}
