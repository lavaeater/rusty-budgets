use crate::User;
use crate::cqrs::framework::StoredEvent;
use crate::cqrs::runtime::{StoredBudgetEvent, UserBudgets};
use crate::models::{Budget, BudgetEvent};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;
use welds::{Syntax, WeldsError, prelude::*};

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

impl From<StoredEvent<Budget, BudgetEvent>> for DbState<PgStoredBudgetEvent> {
    fn from(event: StoredEvent<Budget, BudgetEvent>) -> Self {
        let mut pg_event = PgStoredBudgetEvent::new();
        pg_event.id = event.id;
        pg_event.aggregate_id = event.aggregate_id;
        pg_event.timestamp = event.timestamp;
        pg_event.user_id = event.user_id;
        pg_event.created_at = event.created_at;
        pg_event.data = serde_json::to_value(event.data).unwrap();
        pg_event
    }
}

impl From<&Budget> for DbState<PgBudget> {
    fn from(agg: &Budget) -> Self {
        let mut pg_budget = PgBudget::new();
        pg_budget.id = agg.id;
        pg_budget.last_event = agg.last_event;
        pg_budget.version = agg.version;
        pg_budget.data = serde_json::to_value(agg).expect("Budget must be serializable");
        pg_budget
    }
}

impl From<Budget> for DbState<PgBudget> {
    fn from(agg: Budget) -> Self {
        let mut pg_budget = PgBudget::new();
        pg_budget.id = agg.id;
        pg_budget.last_event = agg.last_event;
        pg_budget.version = agg.version;
        pg_budget.data = serde_json::to_value(agg).expect("Budget must be serializable");
        pg_budget
    }
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

impl From<User> for DbState<PgUser> {
    fn from(value: User) -> Self {
        let mut pg_user = PgUser::new();
        pg_user.id = value.id;
        pg_user.birthday = value.birthday;
        pg_user.email = value.email;
        pg_user.first_name = value.first_name;
        pg_user.last_name = value.last_name;
        pg_user.phone = value.phone;
        pg_user.user_name = value.user_name;
        pg_user
    }
}

impl From<DbState<PgUser>> for User {
    fn from(value: DbState<PgUser>) -> Self {
        User::new(
            &value.user_name,
            &value.email,
            &value.first_name,
            &value.last_name,
            value.phone.clone(),
            value.birthday,
        )
    }
}

impl From<&DbState<PgUser>> for User {
    fn from(value: &DbState<PgUser>) -> Self {
        Self {
            id: value.id,
            user_name: value.user_name.clone(),
            birthday: value.birthday,
            phone: value.phone.clone(),
            last_name: value.last_name.clone(),
            first_name: value.first_name.clone(),
            email: value.email.clone(),
        }
    }
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

impl From<UserBudgets> for DbState<PgUserBudgets> {
    fn from(ub: UserBudgets) -> Self {
        let mut pg_ub = PgUserBudgets::new();
        pg_ub.id = ub.id;
        pg_ub.budgets = serde_json::to_value(ub).expect("Budgets must be serializable");
        pg_ub
    }
}

impl From<DbState<PgUserBudgets>> for UserBudgets {
    fn from(pg_ub: DbState<PgUserBudgets>) -> Self {
        UserBudgets {
            id: pg_ub.id,
            budgets: serde_json::from_value(pg_ub.budgets.clone()).expect("Budgets must be serializable"),
        }
    }
}

impl From<PgUserBudgets> for UserBudgets {
    fn from(pg_ub: PgUserBudgets) -> Self {
        UserBudgets {
            id: pg_ub.id,
            budgets: serde_json::from_value(pg_ub.budgets).expect("Budgets must be serializable"),
        }
    }
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
