//! This crate contains all shared fullstack server functions.
#![allow(unused_imports)]
#![allow(dead_code)]
pub mod api_error;
pub mod cqrs;
pub mod errors;
pub mod events;
pub mod holidays;
pub mod import;
pub mod migrations;
pub mod models;
#[cfg(feature = "server")]
pub mod pg_models;
pub mod time_delta;
pub mod view_models;

#[cfg(feature = "server")]
pub mod db;

#[cfg(feature = "server")]
use dioxus::logger::tracing;

use crate::api_error::RustyError;
pub use crate::models::*;
use dioxus::prelude::*;
use uuid::Uuid;
use view_models::BudgetViewModel;
use view_models::TagSummary;

#[server(endpoint = "create_budget")]
pub async fn create_budget(
    name: String,
    period_id: PeriodId,
    default_budget: Option<bool>,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    let budget_id = db::create_budget(user.id, &name, default_budget.unwrap_or(true)).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "add_actual")]
pub async fn add_actual(
    budget_id: Uuid,
    item_id: Uuid,
    budgeted_amount: Money,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::add_actual(user.id, budget_id, item_id, budgeted_amount, period_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "auto_budget_period")]
pub async fn auto_budget_period(
    budget_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::auto_budget_period(user.id, budget_id, period_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "add_new_actual_item")]
pub async fn add_new_actual_item(
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
    budgeted_amount: Money,
    tx_id: Option<Uuid>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    let item_id = db::add_item(user.id, budget_id, name, item_type).await?;
    let actual_id = db::add_actual(user.id, budget_id, item_id, budgeted_amount, period_id).await?;

    if let Some(tx_id) = tx_id {
        db::connect_transaction(user.id, budget_id, tx_id, Some(actual_id), item_id, period_id, String::new()).await?;
        db::create_rule(user.id, budget_id, tx_id, actual_id).await?;
        db::evaluate_rules(user.id, budget_id).await?;
    }
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "modify_item")]
pub async fn modify_item(
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
    tag_ids: Option<Vec<Uuid>>,
    periodicity: Option<Periodicity>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::modify_item(user.id, budget_id, item_id, name, item_type, tag_ids, periodicity).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "set_item_buffer")]
pub async fn set_item_buffer(
    budget_id: Uuid,
    item_id: Uuid,
    buffer_target: Option<Money>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::set_item_buffer(user.id, budget_id, item_id, buffer_target).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "create_tag")]
pub async fn create_tag(
    budget_id: Uuid,
    name: String,
    periodicity: Periodicity,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::create_tag(user.id, budget_id, name, periodicity).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "get_tags")]
pub async fn get_tags(budget_id: Uuid) -> ServerFnResult<Vec<Tag>> {
    Ok(db::get_tags(budget_id).await?)
}

#[server(endpoint = "modify_tag")]
pub async fn modify_tag(
    budget_id: Uuid,
    tag_id: Uuid,
    name: Option<String>,
    periodicity: Option<Periodicity>,
    deleted: Option<bool>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::modify_tag(user.id, budget_id, tag_id, name, periodicity, deleted).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "modify_actual")]
pub async fn modify_actual(
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Option<Money>,
    actual_amount: Option<Money>,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::modify_actual(user.id, budget_id, actual_id, period_id, budgeted_amount, actual_amount).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "get_default_user")]
pub async fn get_default_user() -> ServerFnResult<User> {
    Ok(db::get_default_user().await?)
}

#[server(endpoint = "get_budget")]
pub async fn get_budget(
    budget_id: Option<Uuid>,
    period_id: PeriodId,
) -> ServerFnResult<Option<BudgetViewModel>> {
    let user = db::get_default_user().await?;
    match budget_id {
        Some(budget_id) => {
            db::evaluate_rules(user.id, budget_id).await?;
            Ok(Some(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id)))
        }
        None => match db::get_default_budget(user.id).await {
            Ok(default_budget) => {
                db::evaluate_rules(user.id, default_budget.id).await?;
                Ok(Some(BudgetViewModel::from_budget(&default_budget, period_id)))
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
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::import_transactions(user.id, budget_id, &file_name).await?;
    db::evaluate_rules(user.id, budget_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "import_transactions_bytes")]
pub async fn import_transactions_bytes(
    budget_id: Uuid,
    file_contents: Vec<u8>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::import_transactions_bytes(user.id, budget_id, file_contents).await?;
    db::evaluate_rules(user.id, budget_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "connect_transaction")]
pub async fn connect_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    budget_item_id: Uuid,
    tag: Option<String>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    let actual_id = db::connect_transaction(user.id, budget_id, tx_id, actual_id, budget_item_id, period_id, tag.unwrap_or_default()).await?;
    db::create_rule(user.id, budget_id, tx_id, actual_id).await?;
    db::evaluate_rules(user.id, budget_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "ignore_transaction")]
pub async fn ignore_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::ignore_transaction(budget_id, user.id, tx_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "adjust_actual_funds")]
pub async fn adjust_actual_funds(
    budget_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::adjust_actual_funds(user.id, budget_id, actual_id, period_id, amount).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "create_allocation")]
pub async fn create_allocation(
    budget_id: Uuid,
    transaction_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    tag: String,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::create_allocation(user.id, budget_id, transaction_id, actual_id, amount, tag).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "delete_allocation")]
pub async fn delete_allocation(
    budget_id: Uuid,
    allocation_id: Uuid,
    transaction_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::delete_allocation(user.id, budget_id, allocation_id, transaction_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "get_next_untagged_transaction")]
pub async fn get_next_untagged_transaction(budget_id: Uuid) -> ServerFnResult<Option<BankTransaction>> {
    Ok(db::get_next_untagged_transaction(budget_id).await?)
}

#[server(endpoint = "get_transactions_for_tag")]
pub async fn get_transactions_for_tag(budget_id: Uuid, tag_id: Uuid) -> ServerFnResult<Vec<BankTransaction>> {
    Ok(db::get_transactions_for_tag(budget_id, tag_id).await?)
}

#[server(endpoint = "get_tagged_transactions")]
pub async fn get_tagged_transactions(budget_id: Uuid, limit: usize, offset: usize) -> ServerFnResult<Vec<BankTransaction>> {
    Ok(db::get_tagged_transactions(budget_id, limit, offset).await?)
}

#[server(endpoint = "get_untagged_transactions")]
pub async fn get_untagged_transactions(budget_id: Uuid, limit: usize) -> ServerFnResult<Vec<BankTransaction>> {
    Ok(db::get_untagged_transactions(budget_id, limit).await?)
}

#[server(endpoint = "tag_transaction")]
pub async fn tag_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    tag_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::tag_transaction(user.id, budget_id, tx_id, tag_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "preview_rule_matches")]
pub async fn preview_rule_matches(budget_id: Uuid, tx_id: Uuid) -> ServerFnResult<Vec<BankTransaction>> {
    Ok(db::preview_rule_matches(budget_id, tx_id).await?)
}

#[server(endpoint = "reject_transfer_pair")]
pub async fn reject_transfer_pair(
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::reject_transfer_pair(user.id, budget_id, outgoing_tx_id, incoming_tx_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "resolve_transfer_pair")]
pub async fn resolve_transfer_pair(
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
    tag_id: Option<Uuid>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::resolve_transfer_pair(user.id, budget_id, outgoing_tx_id, incoming_tx_id, tag_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "get_unbudgeted_tag_summaries")]
pub async fn get_unbudgeted_tag_summaries(budget_id: Uuid) -> ServerFnResult<Vec<TagSummary>> {
    use std::collections::HashSet;
    let budget = db::get_budget(budget_id).await?;
    let budgeted_tag_ids: HashSet<Uuid> = budget
        .items
        .iter()
        .flat_map(|item| item.tag_ids.iter().copied())
        .collect();
    Ok(budget
        .get_tag_summaries()
        .into_iter()
        .filter(|ts| !budgeted_tag_ids.contains(&ts.tag_id))
        .collect())
}

#[server(endpoint = "create_budget_item")]
pub async fn create_budget_item(
    budget_id: Uuid,
    name: String,
    budgeting_type: BudgetingType,
    tag_ids: Vec<Uuid>,
    budgeted_amount: Option<Money>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    let item_id = db::add_item(user.id, budget_id, name, budgeting_type).await?;
    if !tag_ids.is_empty() {
        db::modify_item(user.id, budget_id, item_id, None, None, Some(tag_ids), None).await?;
    }
    if let Some(amount) = budgeted_amount
        && !amount.is_zero()
    {
        db::add_actual(user.id, budget_id, item_id, amount, period_id).await?;
    }
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "delete_rule")]
pub async fn delete_rule(
    budget_id: Uuid,
    rule_id: Uuid,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::delete_rule(user.id, budget_id, rule_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}

#[server(endpoint = "update_rule")]
pub async fn update_rule(
    budget_id: Uuid,
    rule_id: Uuid,
    transaction_key: Vec<String>,
    period_id: PeriodId,
) -> ServerFnResult<BudgetViewModel> {
    let user = db::get_default_user().await?;
    db::modify_rule(user.id, budget_id, rule_id, transaction_key).await?;
    db::evaluate_tag_rules(user.id, budget_id).await?;
    Ok(BudgetViewModel::from_budget(&db::get_budget(budget_id).await?, period_id))
}
