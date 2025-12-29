//! This crate contains all shared fullstack server functions.
#![allow(unused_imports)]
#![allow(dead_code)]
pub mod api_error;
pub mod cqrs;
pub mod events;
pub mod holidays;
pub mod import;
pub mod models;
pub mod time_delta;
pub mod view_models;

#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
use dioxus::logger::tracing;

use crate::api_error::RustyError;
use crate::import::ImportError;
use crate::models::*;
use chrono::Utc;
use dioxus::prelude::*;
use joydb::JoydbError;
use models::*;
use uuid::Uuid;
use view_models::BudgetItemViewModel;
use view_models::BudgetViewModel;
use view_models::TransactionViewModel;

#[server(endpoint = "create_budget")]
pub async fn create_budget(
    name: String,
    period_id: PeriodId,
    default_budget: Option<bool>,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user(None)?;
    let budget_id = db::create_budget(user.id, &name, default_budget.unwrap_or(true))?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "add_actual")]
pub async fn add_actual(
    budget_id: Uuid,
    item_id: Uuid,
    budgeted_amount: Money,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    let _ = db::add_actual(user.id, budget_id, item_id, budgeted_amount, period_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "auto_budget_period")]
pub async fn auto_budget_period(
    budget_id: Uuid,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    db::auto_budget_period(user.id, budget_id, period_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "add_new_actual_item")]
pub async fn add_new_actual_item(
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
    budgeted_amount: Money,
    tx_id: Option<Uuid>,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let item_id = db::add_item(user.id, budget_id, name, item_type)?;
    info!("We have a new item with Id: {}", item_id);

    let actual_id = db::add_actual(user.id, budget_id, item_id, budgeted_amount, period_id)?;
    info!("We have a new actual with Id: {}", actual_id);

    match tx_id {
        Some(tx_id) => {
            let _ = db::connect_transaction(
                user.id,
                budget_id,
                tx_id,
                Some(actual_id),
                item_id,
                period_id,
            )?;

            let _ = db::create_rule(user.id, budget_id, tx_id, actual_id)?;
            let _ = db::evaluate_rules(user.id, budget_id)?;
            Ok(BudgetViewModel::from_budget(
                &db::get_budget(budget_id)?,
                period_id,
            ))
        }
        None => Ok(BudgetViewModel::from_budget(
            &db::get_budget(budget_id)?,
            period_id,
        )),
    }
}

#[server(endpoint = "modify_item")]
pub async fn modify_item(
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    let _ = db::modify_item(user.id, budget_id, item_id, name, item_type)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "modify_actual")]
pub async fn modify_actual(
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Option<Money>,
    actual_amount: Option<Money>,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    let _ = db::modify_actual(
        user.id,
        budget_id,
        actual_id,
        period_id,
        budgeted_amount,
        actual_amount,
    )?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "get_default_user")]
pub async fn get_default_user() -> Result<User, ServerFnError> {
    Ok(db::get_default_user(None)?)
}

#[server(endpoint = "get_budget")]
pub async fn get_budget(
    budget_id: Option<Uuid>,
    period_id: PeriodId,
) -> Result<Option<BudgetViewModel>, ServerFnError> {
    let user = db::get_default_user(None)?;
    match budget_id {
        Some(budget_id) => {
            _ = db::evaluate_rules(user.id, budget_id)?;
            Ok(Some(BudgetViewModel::from_budget(
                &db::get_budget(budget_id)?,
                period_id,
            )))
        }
        None => match db::get_default_budget(user.id) {
            Ok(default_budget) => {
                let _ = db::evaluate_rules(user.id, default_budget.id)?;
                Ok(Some(BudgetViewModel::from_budget(
                    &default_budget,
                    period_id,
                )))
            }
            Err(_) => Ok(None),
        },
    }
}

#[server(endpoint = "import_transactions")]
pub async fn import_transactions(
    budget_id: Uuid,
    file_name: String,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let _ = db::import_transactions(user.id, budget_id, &file_name)?;
    let _ = db::evaluate_rules(user.id, budget_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "import_transactions_bytes")]
pub async fn import_transactions_bytes(
    budget_id: Uuid,
    file_contents: Vec<u8>,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let _ = db::import_transactions_bytes(user.id, budget_id, file_contents)?;
    let _ = db::evaluate_rules(user.id, budget_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "connect_transaction")]
pub async fn connect_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    budget_item_id: Uuid,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let actual_id = db::connect_transaction(
        user.id,
        budget_id,
        tx_id,
        actual_id,
        budget_item_id,
        period_id,
    )?;
    let _ = db::create_rule(user.id, budget_id, tx_id, actual_id)?;
    let _ = db::evaluate_rules(user.id, budget_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "ignore_transaction")]
pub async fn ignore_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let _ = db::ignore_transaction(budget_id, user.id, tx_id)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}

#[server(endpoint = "adjust_actual_funds")]
pub async fn adjust_actual_funds(
    budget_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    period_id: PeriodId,
) -> Result<BudgetViewModel, ServerFnError> {
    let user = db::get_default_user(None)?;
    let _ = db::adjust_actual_funds(user.id, budget_id, actual_id, period_id, amount)?;
    Ok(BudgetViewModel::from_budget(
        &db::get_budget(budget_id)?,
        period_id,
    ))
}
