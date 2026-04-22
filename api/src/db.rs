pub const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

use crate::api_error::RustyError;
use crate::cqrs::framework::AsyncRuntime;
use crate::cqrs::runtime::{AsyncBudgetCommandsTrait, PgRuntime, create_runtime};
use crate::import::{import_from_path, import_from_skandia_excel_bytes};
use crate::models::*;
use chrono::NaiveDate;
use dioxus::logger::tracing;
use dioxus::logger::tracing::error;
use dioxus::logger::tracing::info;
use std::env;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use uuid::Uuid;

fn get_data_file() -> PathBuf {
    env::var("DATA_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            info!("DATA_FILE not set, using default data.json");
            PathBuf::from("data.json")
        })
}

static PG: OnceCell<PgRuntime> = OnceCell::const_new();

async fn runtime() -> &'static PgRuntime {
    PG.get_or_init(|| async {
        info!("Init Postgres runtime");
        let rt = create_runtime().await;
        match rt.get_default_user().await {
            Ok(_) => info!("Default user exists"),
            Err(e) => {
                error!(error = %e, "Could not get default user");
                panic!("Could not get default user");
            }
        }
        rt
    })
    .await
}

pub async fn user_exists(email: &str) -> Result<bool, RustyError> {
    runtime().await.user_exists(email).await
}

pub async fn get_default_user() -> Result<User, RustyError> {
    runtime().await.get_default_user().await
}

pub async fn get_default_budget(user_id: Uuid) -> Result<Budget, RustyError> {
    runtime().await.get_default_budget(user_id).await
}

pub async fn get_budget(budget_id: Uuid) -> Result<Budget, RustyError> {
    runtime().await.load(budget_id).await
}

pub async fn add_budget_to_user(
    user_id: Uuid,
    budget_id: Uuid,
    default: bool,
) -> Result<Uuid, RustyError> {
    runtime().await.add_budget_to_user(user_id, budget_id, default).await
}

pub async fn create_budget(user_id: Uuid, name: &str, default_budget: bool) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget_id = rt
        .create_budget(user_id, name, default_budget, MonthBeginsOn::default(), Currency::SEK)
        .await?;
    rt.add_budget_to_user(user_id, budget_id, default_budget).await?;
    Ok(budget_id)
}

pub async fn import_transactions(
    user_id: Uuid,
    budget_id: Uuid,
    file_name: &str,
) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    import_from_path(file_name, user_id, budget_id, rt)
        .await
        .map_err(RustyError::ImportError)?;
    Ok(budget_id)
}

pub async fn import_transactions_bytes(
    user_id: Uuid,
    budget_id: Uuid,
    bytes: Vec<u8>,
) -> Result<Uuid, RustyError> {
    info!("Importing transaction from bytes");
    let rt = runtime().await;
    import_from_skandia_excel_bytes(rt, user_id, budget_id, bytes)
        .await
        .map_err(RustyError::ImportError)?;
    Ok(budget_id)
}

pub async fn add_item(
    user_id: Uuid,
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
) -> Result<Uuid, RustyError> {
    runtime().await.add_item(user_id, budget_id, name, item_type).await
}

pub async fn evaluate_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    for rule_match in budget.evaluate_rules().iter() {
        let tx_id = rule_match.tx_id;
        let amount = rule_match.amount;

        let actual_id = if let Some(actual_id) = rule_match.actual_id {
            actual_id
        } else if let Some(item_id) = rule_match.item_id {
            let period_id = budget.get_period_for_transaction(tx_id).unwrap().id;
            match rt
                .add_actual(user_id, budget_id, item_id, Money::zero(budget.currency), period_id)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    error!(error = %e, "Could not create actual for tx {}", tx_id);
                    continue;
                }
            }
        } else {
            tracing::warn!("No actual or item found for transaction {}", tx_id);
            continue;
        };

        match rt
            .create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new())
            .await
        {
            Ok(_) => info!("Allocated tx {} to actual {}", tx_id, actual_id),
            Err(e) => error!(error = %e, "Could not allocate tx {} to actual {}", tx_id, actual_id),
        }
    }
    Ok(budget_id)
}

pub async fn add_actual(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    budget_amount: Money,
    period_id: PeriodId,
) -> Result<Uuid, RustyError> {
    runtime().await.add_actual(user_id, budget_id, item_id, budget_amount, period_id).await
}

pub async fn modify_item(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
    tag_ids: Option<Vec<Uuid>>,
    periodicity: Option<Periodicity>,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .modify_item(user_id, budget_id, item_id, name, item_type, tag_ids, periodicity)
        .await
}

pub async fn modify_actual(
    user_id: Uuid,
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    budgeted_amount: Option<Money>,
    actual_amount: Option<Money>,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .modify_actual(user_id, budget_id, actual_id, period_id, budgeted_amount, actual_amount)
        .await
}

pub async fn ensure_account(
    user_id: Uuid,
    budget_id: Uuid,
    account_number: &str,
    description: &str,
) -> Result<Uuid, RustyError> {
    runtime().await.ensure_account(user_id, budget_id, account_number, description).await
}

pub async fn connect_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    item_id: Uuid,
    period_id: PeriodId,
    tag: String,
) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;

    let actual_id = match actual_id {
        None => {
            rt.add_actual(user_id, budget_id, item_id, Money::zero(budget.currency), period_id)
                .await?
        }
        Some(id) => id,
    };

    let amount = budget
        .get_transaction(tx_id)
        .map(|tx| tx.amount)
        .ok_or_else(|| RustyError::ItemNotFound(tx_id.to_string(), "Transaction not found".to_string()))?;

    rt.create_allocation(user_id, budget_id, tx_id, actual_id, amount, tag).await?;
    Ok(actual_id)
}

pub async fn ignore_transaction(budget_id: Uuid, user_id: Uuid, tx_id: Uuid) -> Result<Uuid, RustyError> {
    runtime().await.ignore_transaction(budget_id, tx_id, user_id).await?;
    Ok(budget_id)
}

pub async fn adjust_actual_funds(
    user_id: Uuid,
    budget_id: Uuid,
    actual_id: Uuid,
    period_id: PeriodId,
    amount: Money,
) -> Result<Uuid, RustyError> {
    runtime().await.adjust_budgeted_amount(user_id, budget_id, actual_id, period_id, amount).await?;
    Ok(budget_id)
}

pub async fn create_allocation(
    user_id: Uuid,
    budget_id: Uuid,
    transaction_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    tag: String,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .create_allocation(user_id, budget_id, transaction_id, actual_id, amount, tag)
        .await
}

pub async fn delete_allocation(
    user_id: Uuid,
    budget_id: Uuid,
    allocation_id: Uuid,
    transaction_id: Uuid,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .delete_allocation(user_id, budget_id, allocation_id, transaction_id)
        .await
}

pub async fn undo_last(budget_id: Uuid) -> Result<bool, RustyError> {
    runtime().await.undo_last(budget_id).await
}

pub async fn create_rule(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Uuid,
) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    let transaction = budget.get_transaction(tx_id).ok_or(RustyError::ItemNotFound(
        tx_id.to_string(),
        "Transaction not found".to_string(),
    ))?;
    let period_id = PeriodId::from_date(transaction.date, budget.month_begins_on());
    let period = budget.get_period(period_id).ok_or(RustyError::ItemNotFound(
        period_id.to_string(),
        "Period not found".to_string(),
    ))?;
    let item = period.get_actual(actual_id).ok_or(RustyError::ItemNotFound(
        actual_id.to_string(),
        "Actual item not found".to_string(),
    ))?;
    let transaction_key = MatchRule::create_transaction_key(transaction);
    let item_key = MatchRule::create_item_key(item);
    rt.add_rule(user_id, budget.id, transaction_key, item_key, true, None).await?;
    Ok(budget.id)
}

pub async fn create_user(
    user_name: &str,
    email: &str,
    first_name: &str,
    last_name: &str,
    phone: Option<String>,
    birthday: Option<NaiveDate>,
) -> Result<User, RustyError> {
    runtime()
        .await
        .create_user(user_name, email, first_name, last_name, phone, birthday)
        .await
}

pub async fn auto_budget_period(
    user_id: Uuid,
    budget_id: Uuid,
    period_id: PeriodId,
) -> Result<(), RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    let period = budget.get_period(period_id).ok_or(RustyError::ItemNotFound(
        period_id.to_string(),
        "Period not found".to_string(),
    ))?;
    info!("Auto budgeting period {}", period_id);
    info!("Number of items: {}", period.actual_items.len());
    for actual in &period.actual_items {
        if actual.budgeted_amount.is_zero() {
            match rt
                .modify_actual(user_id, budget_id, actual.id, period_id, Some(actual.actual_amount), None)
                .await
            {
                Ok(_) => {}
                Err(e) => error!(error = %e, "Could not modify actual"),
            }
        }
    }
    Ok(())
}

pub async fn create_tag(
    user_id: Uuid,
    budget_id: Uuid,
    name: String,
    periodicity: Periodicity,
) -> Result<Uuid, RustyError> {
    runtime().await.create_tag(user_id, budget_id, name, periodicity).await
}

pub async fn get_tags(budget_id: Uuid) -> Result<Vec<Tag>, RustyError> {
    Ok(get_budget(budget_id).await?.tags)
}

pub async fn modify_tag(
    user_id: Uuid,
    budget_id: Uuid,
    tag_id: Uuid,
    name: Option<String>,
    periodicity: Option<Periodicity>,
    deleted: Option<bool>,
) -> Result<Uuid, RustyError> {
    runtime().await.modify_tag(user_id, budget_id, tag_id, name, periodicity, deleted).await
}

pub async fn get_next_untagged_transaction(budget_id: Uuid) -> Result<Option<BankTransaction>, RustyError> {
    Ok(get_budget(budget_id).await?.get_next_untagged_transaction().cloned())
}

pub async fn get_transactions_for_tag(
    budget_id: Uuid,
    tag_id: Uuid,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id).await?;
    let mut txs: Vec<BankTransaction> = budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|tx| tx.tag_id == Some(tag_id) && !tx.ignored)
        .cloned()
        .collect();
    txs.sort_by_key(|b| std::cmp::Reverse(b.date));
    Ok(txs)
}

pub async fn get_tagged_transactions(
    budget_id: Uuid,
    limit: usize,
    offset: usize,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id).await?;
    let mut txs: Vec<BankTransaction> = budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|tx| tx.tag_id.is_some() && !tx.ignored)
        .cloned()
        .collect();
    txs.sort_by_key(|b| std::cmp::Reverse(b.date));
    Ok(txs.into_iter().skip(offset).take(limit).collect())
}

pub async fn get_untagged_transactions(budget_id: Uuid, limit: usize) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id).await?;
    let transfer_ids: std::collections::HashSet<Uuid> = budget
        .potential_internal_transfers()
        .into_iter()
        .flat_map(|(a, b)| [a, b])
        .collect();
    Ok(budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|tx| tx.tag_id.is_none() && !tx.ignored && !transfer_ids.contains(&tx.id))
        .take(limit)
        .cloned()
        .collect())
}

pub async fn reject_transfer_pair(
    user_id: Uuid,
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .reject_transfer_pair(user_id, budget_id, outgoing_tx_id, incoming_tx_id)
        .await
}

pub async fn resolve_transfer_pair(
    user_id: Uuid,
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
    tag_id: Option<Uuid>,
) -> Result<Uuid, RustyError> {
    if let Some(tag_id) = tag_id {
        tag_transaction(user_id, budget_id, outgoing_tx_id, tag_id).await?;
    } else {
        ignore_transaction(budget_id, user_id, outgoing_tx_id).await?;
    }
    ignore_transaction(budget_id, user_id, incoming_tx_id).await?;
    Ok(budget_id)
}

pub async fn tag_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    tag_id: Uuid,
) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    let rt = runtime().await;
    rt.tag_transaction(user_id, budget_id, tx_id, tag_id).await?;
    tracing::info!("[perf] tag_transaction/tag_event: {:?}", t.elapsed());
    let budget = rt.load(budget_id).await?;
    tracing::info!("[perf] tag_transaction/load_for_rule_check: {:?}", t.elapsed());
    let tx = budget.get_transaction(tx_id).ok_or(RustyError::ItemNotFound(
        tx_id.to_string(),
        "Transaction not found".to_string(),
    ))?;
    let transaction_key = MatchRule::create_transaction_key(tx);
    let rule_exists = budget
        .match_rules
        .iter()
        .any(|r| r.transaction_key == transaction_key && r.tag_id == Some(tag_id));
    if !rule_exists {
        rt.add_rule(user_id, budget_id, transaction_key, Vec::new(), true, Some(tag_id)).await?;
        tracing::info!("[perf] tag_transaction/add_rule: {:?}", t.elapsed());
    }
    evaluate_tag_rules(user_id, budget_id).await?;
    tracing::info!("[perf] tag_transaction/total: {:?}", t.elapsed());
    Ok(budget_id)
}

pub async fn evaluate_tag_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    let matches = budget.evaluate_tag_rules();
    let match_count = matches.len();
    info!("[perf] evaluate_tag_rules: {} matches found in {:?}", match_count, t.elapsed());
    for (tx_id, tag_id) in matches {
        match rt.tag_transaction(user_id, budget_id, tx_id, tag_id).await {
            Ok(_) => info!("Tagged a transaction, bro!"),
            Err(e) => error!(error = %e, "Could not tag transaction {} with tag {}", tx_id, tag_id),
        }
    }
    info!("[perf] evaluate_tag_rules/total (applied {} tags): {:?}", match_count, t.elapsed());
    Ok(budget_id)
}

pub async fn preview_rule_matches(budget_id: Uuid, tx_id: Uuid) -> Result<Vec<BankTransaction>, RustyError> {
    Ok(get_budget(budget_id).await?.preview_rule_matches(tx_id))
}

pub async fn modify_rule(
    user_id: Uuid,
    budget_id: Uuid,
    rule_id: Uuid,
    transaction_key: Vec<String>,
) -> Result<Uuid, RustyError> {
    runtime().await.modify_rule(user_id, budget_id, rule_id, transaction_key).await
}

pub async fn delete_rule(user_id: Uuid, budget_id: Uuid, rule_id: Uuid) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    let deleted_tag_id = budget.match_rules.iter().find(|r| r.id == rule_id).and_then(|r| r.tag_id);
    rt.delete_rule(user_id, budget_id, rule_id).await?;

    if let Some(tag_id) = deleted_tag_id {
        let budget = rt.load(budget_id).await?;
        let transactions_to_check: Vec<Uuid> = budget
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|tx| tx.tag_id == Some(tag_id) && !tx.ignored)
            .map(|tx| tx.id)
            .collect();

        for tx_id in transactions_to_check {
            let budget = rt.load(budget_id).await?;
            if let Some(tx) = budget.get_transaction(tx_id) {
                let still_matches = budget
                    .match_rules
                    .iter()
                    .any(|r| r.tag_id == Some(tag_id) && r.matches_transaction(tx));
                if !still_matches {
                    match rt.untag_transaction(user_id, budget_id, tx_id).await {
                        Ok(_) => info!("Untagged transaction {} after rule deletion", tx_id),
                        Err(e) => error!(error = %e, "Failed to untag transaction {} after rule deletion", tx_id),
                    }
                }
            }
        }
    }

    Ok(budget_id)
}

pub async fn set_item_buffer(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    buffer_target: Option<Money>,
) -> Result<Uuid, RustyError> {
    runtime().await.set_item_buffer(user_id, budget_id, item_id, buffer_target).await
}
