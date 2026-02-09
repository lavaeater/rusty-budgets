const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

use crate::api_error::RustyError;
use crate::cqrs::framework::{CommandError, Runtime};
use crate::cqrs::runtime::{Db, JoyDbBudgetRuntime, UserBudgets};
use crate::events::TransactionConnected;
use crate::import::{import_from_path, import_from_skandia_excel, import_from_skandia_excel_bytes};
use crate::models::*;
use crate::models::*;
use chrono::NaiveDate;
use dioxus::logger::tracing;
use dioxus::logger::tracing::error;
use dioxus::logger::tracing::info;
use joydb::JoydbError;
use once_cell::sync::Lazy;
use std::env;
use std::path::PathBuf;
use uuid::Uuid;

fn get_data_file() -> PathBuf {
    env::var("DATA_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            info!("DATA_FILE not set, using default data.json");
            PathBuf::from("data.json") })
}

pub static CLIENT: Lazy<JoyDbBudgetRuntime> = Lazy::new(|| {
    info!("Init DB Client");

    let client = JoyDbBudgetRuntime::new(get_data_file());
    // Run migrations
    info!("Insert Default Data");
    match get_default_user(Some(&client.db)) {
        Ok(_) => {
            info!("Default user exists");
        }
        Err(e) => {
            error!(error = %e, "Could not get default user");
            panic!("Could not get default user");
        }
    }
    client
});

fn with_client(client: Option<&Db>) -> &Db {
    if let Some(c) = client { c } else { &CLIENT.db }
}

fn with_runtime(client: Option<&JoyDbBudgetRuntime>) -> &JoyDbBudgetRuntime {
    if let Some(c) = client { c } else { &CLIENT }
}

pub fn user_exists(email: &str, client: Option<&Db>) -> Result<bool, RustyError> {
    let users = with_client(client).get_all_by(|u: &User| u.email == email)?;
    Ok(!users.is_empty())
}

pub fn get_default_user(client: Option<&Db>) -> Result<User, RustyError> {
    match with_client(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
        Ok(mut users) => {
            if users.is_empty() {
                create_user(
                    "tommie",
                    DEFAULT_USER_EMAIL,
                    "Tommie",
                    "Nygren",
                    Some("0704382781".to_string()),
                    Some(
                        NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                    ),
                    client,
                )
            } else {
                Ok(users.remove(0))
            }
        }
        Err(e) => {
            error!(error = %e, "Could not get default user");
            Err(RustyError::JoydbError(e))
        }
    }
}

pub fn get_default_budget(user_id: Uuid) -> Result<Budget, RustyError> {
    let user_budgets = with_client(None).get::<UserBudgets>(&user_id)?;
    match user_budgets {
        None => {
            info!("User has no budgets");
            Err(RustyError::DefaultBudgetNotFound)
        }
        Some(b) => match b.budgets.iter().find(|(_, default)| *default) {
            Some((budget_id, _)) => Ok(with_runtime(None).load(*budget_id)?),
            None => {
                info!("User had budgets but none were default");
                Err(RustyError::DefaultBudgetNotFound)
            }
        },
    }
}

//THis one should evaluate the rules!
pub fn get_budget(budget_id: Uuid) -> Result<Budget, RustyError> {
    let budget = with_runtime(None).load(budget_id)?;
    Ok(budget)
}

pub fn add_budget_to_user(
    user_id: Uuid,
    budget_id: Uuid,
    default: bool,
) -> Result<Uuid, RustyError> {
    let user_budgets = with_client(None).get::<UserBudgets>(&user_id)?;
    match user_budgets {
        None => {
            match with_client(None).insert(&UserBudgets {
                id: user_id,
                budgets: vec![(budget_id, default)],
            }).map(|_| user_id) {
                Ok(_) => Ok(user_id),
                Err(e) => Err(RustyError::JoydbError(e)),
            }
        }
        Some(list) => {
            if !list.budgets.contains(&(budget_id, default)) {
                let mut budgets = list.budgets.clone();
                budgets.push((budget_id, default));
                let list = UserBudgets {
                    id: user_id,
                    budgets,
                };
                match with_client(None).upsert(&list) {
                    Ok(_) => Ok(user_id),
                    Err(e) => Err(RustyError::JoydbError(e)),
                }
            } else {
                Ok(user_id)
            }
        }
    }
}

pub fn create_budget(
    user_id: Uuid,
    name: &str,
    default_budget: bool,
) -> Result<Uuid, RustyError> {
    let budget_id = with_runtime(None).create_budget(
        user_id,
        name,
        default_budget,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;
    add_budget_to_user(user_id, budget_id, default_budget)?;
    Ok(budget_id)
}

pub fn import_transactions(
    user_id: Uuid,
    budget_id: Uuid,
    file_name: &str,
) -> Result<Uuid, RustyError> {
    let runtime = with_runtime(None);
    let _ = import_from_path(file_name, user_id, budget_id, runtime)?;
    Ok(budget_id)
}

pub fn import_transactions_bytes(
    user_id: Uuid,
    budget_id: Uuid,
    bytes: Vec<u8>,
) -> Result<Uuid, RustyError> {
    info!("Importing transaction from bytes");
    let runtime = with_runtime(None);
    let _ = import_from_skandia_excel_bytes(runtime, user_id, budget_id, bytes)?;
    Ok(budget_id)
}

pub fn add_item(
    user_id: Uuid,
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
) -> Result<Uuid, RustyError> {
    with_runtime(None).add_item(user_id, budget_id, name, item_type)
}

pub fn evaluate_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let budget = get_budget(budget_id)?;
    for (tx_id, actual_id, item_id) in budget.evaluate_rules().iter() {
        if actual_id.is_none() && item_id.is_none() {
            tracing::warn!("No actual or item found for transaction {}", tx_id);
            continue;
        } else if actual_id.is_none() && item_id.is_some() {
            tracing::warn!("No actual found for transaction {}", tx_id);
            let period_id = budget.get_period_for_transaction(*tx_id).unwrap().id;
            match connect_transaction(
                user_id,
                budget_id,
                *tx_id,
                None,
                item_id.unwrap(),
                period_id,
            ) {
                Ok(_) => {
                    info!("Connected tx {:?} with actual item {:?}", tx_id, actual_id);
                }
                Err(e) => {
                    error!(error = %e, "Could not connect tx {:?} with actual item {:?}", tx_id, actual_id);
                }
            }
        } else if actual_id.is_some() {
            match with_runtime(None).connect_transaction(
                user_id,
                budget_id,
                *tx_id,
                actual_id.unwrap(),
            ) {
                Ok(_) => {
                    info!("Connected tx {:?} with actual item {:?}", tx_id, actual_id);
                }
                Err(e) => {
                    error!(error = %e, "Could not connect tx {:?} with actual item {:?}", tx_id, actual_id);
                }
            }
        }
    }
    Ok(budget_id)
}

pub fn add_actual(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    budget_amount: Money,
    period_id: PeriodId,
) -> Result<Uuid, RustyError> {
    with_runtime(None).add_actual(user_id, budget_id, item_id, budget_amount, period_id)
}

pub fn modify_item(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).modify_item(user_id, budget_id, item_id, name, item_type)
}

/*
    pub budget_id: Uuid,
pub actual_id: Uuid,
pub period_id: PeriodId,
pub budgeted_amount: Option<Money>,
pub actual_amount: Option<Money>,
pub notes: Option<String>,
pub tags: Option<Vec<String>>,
 */
pub fn modify_actual(
    user_id: Uuid,
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Option<Money>,
    actual_amount: Option<Money>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).modify_actual(
        user_id,
        budget_id,
        actual_id,
        period_id,
        budgeted_amount,
        actual_amount,
    )
}

pub fn connect_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    item_id: Uuid,
    period_id: PeriodId,
) -> Result<Uuid, RustyError> {
    let actual_id = match actual_id {
        None => {
            with_runtime(None).add_actual(
                user_id,
                budget_id,
                item_id,
                Money::zero(Currency::default()),
                period_id,
            )?
        }
        Some(actual_id) => actual_id,
    };
    with_runtime(None).connect_transaction(user_id, budget_id, tx_id, actual_id)?;
    Ok(actual_id)
}

pub fn ignore_transaction(
    budget_id: Uuid,
    user_id: Uuid,
    tx_id: Uuid,
) -> Result<Uuid, RustyError> {
    with_runtime(None).ignore_transaction(budget_id, tx_id, user_id)?;
    Ok(budget_id)
}

pub fn adjust_actual_funds(
    user_id: Uuid,
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    amount: Money,
) -> Result<Uuid, RustyError> {
    with_runtime(None)
        .adjust_budgeted_amount(user_id, budget_id, actual_id, period_id, amount)?;
    Ok(budget_id)
}

pub fn create_rule(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Uuid,
) -> Result<Uuid, RustyError> {
    let budget = get_budget(budget_id)?;
    let transaction = budget
        .get_transaction(tx_id)
        .ok_or(RustyError::ItemNotFound(
            tx_id.to_string(),
            "Transaction not found".to_string(),
        ))?;
    let period_id = PeriodId::from_date(transaction.date, budget.month_begins_on());
    let period = budget
        .get_period(period_id)
        .ok_or(RustyError::ItemNotFound(
            period_id.to_string(),
            "Period not found".to_string(),
        ))?;
    let item = period
        .get_actual(actual_id)
        .ok_or(RustyError::ItemNotFound(
            actual_id.to_string(),
            "Actual item not found".to_string(),
        ))?;
    let transaction_key = MatchRule::create_transaction_key(transaction);
    let item_key = MatchRule::create_item_key(item);
    let always_apply = true;

    with_runtime(None).add_rule(user_id, budget.id, transaction_key, item_key, always_apply)?;
    Ok(budget.id)
}

pub fn create_user(
    user_name: &str,
    email: &str,
    first_name: &str,
    last_name: &str,
    phone: Option<String>,
    birthday: Option<NaiveDate>,
    client: Option<&Db>,
) -> Result<User, RustyError> {
    let user = User::new(user_name, email, first_name, last_name, phone, birthday);
    with_client(client).insert(&user)?;
    Ok(user)
}

pub(crate) fn auto_budget_period(
    user_id: Uuid,
    budget_id: Uuid,
    period_id: PeriodId,
) -> Result<(), RustyError> {
    let budget = get_budget(budget_id)?;
    let period = budget
        .get_period(period_id)
        .ok_or(RustyError::ItemNotFound(
            period_id.to_string(),
            "Period not found".to_string(),
        ))?;
    info!("Auto budgeting period {}", period_id);
    info!("Number of items: {}", period.actual_items.len());
    period.actual_items.iter().for_each(|actual| {
        let budgeted_amount = actual.budgeted_amount;
        if budgeted_amount.is_zero() {
            let actual_amount = actual.actual_amount;
            match modify_actual(
                user_id,
                budget_id,
                actual.id,
                period_id,
                Some(actual_amount),
                None,
            ) {
                Ok(_) => {}
                Err(e) => {
                    error!(error = %e, "Could not modify actual");
                }
            }
        }
    });
    Ok(())
}