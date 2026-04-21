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
            PathBuf::from("data.json")
        })
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
                    Some(NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default()),
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
            match with_client(None)
                .insert(&UserBudgets {
                    id: user_id,
                    budgets: vec![(budget_id, default)],
                })
                .map(|_| user_id)
            {
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

pub fn create_budget(user_id: Uuid, name: &str, default_budget: bool) -> Result<Uuid, RustyError> {
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
    for rule_match in budget.evaluate_rules().iter() {
        let tx_id = rule_match.tx_id;
        let amount = rule_match.amount;

        let actual_id = if let Some(actual_id) = rule_match.actual_id {
            actual_id
        } else if let Some(item_id) = rule_match.item_id {
            let period_id = budget.get_period_for_transaction(tx_id).unwrap().id;
            match add_actual(
                user_id,
                budget_id,
                item_id,
                Money::zero(budget.currency),
                period_id,
            ) {
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

        match create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new()) {
            Ok(_) => {
                info!("Allocated tx {} to actual {}", tx_id, actual_id);
            }
            Err(e) => {
                error!(error = %e, "Could not allocate tx {} to actual {}", tx_id, actual_id);
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
    tag_ids: Option<Vec<Uuid>>,
    periodicity: Option<Periodicity>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).modify_item(
        user_id,
        budget_id,
        item_id,
        name,
        item_type,
        tag_ids,
        periodicity,
    )
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

pub fn ensure_account(
    user_id: Uuid,
    budget_id: Uuid,
    account_number: &str,
    description: &str,
) -> Result<Uuid, RustyError> {
    with_runtime(None).ensure_account(user_id, budget_id, account_number, description)
}

pub fn connect_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Option<Uuid>,
    item_id: Uuid,
    period_id: PeriodId,
    tag: String,
) -> Result<Uuid, RustyError> {
    let budget = get_budget(budget_id)?;

    let actual_id = match actual_id {
        None => with_runtime(None).add_actual(
            user_id,
            budget_id,
            item_id,
            Money::zero(budget.currency),
            period_id,
        )?,
        Some(actual_id) => actual_id,
    };

    let amount = budget
        .get_transaction(tx_id)
        .map(|tx| tx.amount)
        .ok_or_else(|| {
            RustyError::ItemNotFound(tx_id.to_string(), "Transaction not found".to_string())
        })?;

    create_allocation(user_id, budget_id, tx_id, actual_id, amount, tag)?;
    Ok(actual_id)
}

pub fn ignore_transaction(budget_id: Uuid, user_id: Uuid, tx_id: Uuid) -> Result<Uuid, RustyError> {
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
    with_runtime(None).adjust_budgeted_amount(user_id, budget_id, actual_id, period_id, amount)?;
    Ok(budget_id)
}

pub fn create_allocation(
    user_id: Uuid,
    budget_id: Uuid,
    transaction_id: Uuid,
    actual_id: Uuid,
    amount: Money,
    tag: String,
) -> Result<Uuid, RustyError> {
    with_runtime(None).create_allocation(user_id, budget_id, transaction_id, actual_id, amount, tag)
}

pub fn delete_allocation(
    user_id: Uuid,
    budget_id: Uuid,
    allocation_id: Uuid,
    transaction_id: Uuid,
) -> Result<Uuid, RustyError> {
    with_runtime(None).delete_allocation(user_id, budget_id, allocation_id, transaction_id)
}

pub fn undo_last(budget_id: Uuid) -> Result<bool, RustyError> {
    with_runtime(None).undo_last(budget_id)
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

    with_runtime(None).add_rule(
        user_id,
        budget.id,
        transaction_key,
        item_key,
        always_apply,
        None,
    )?;
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

pub fn create_tag(
    user_id: Uuid,
    budget_id: Uuid,
    name: String,
    periodicity: Periodicity,
) -> Result<Uuid, RustyError> {
    with_runtime(None).create_tag(user_id, budget_id, name, periodicity)
}

pub fn get_tags(budget_id: Uuid) -> Result<Vec<Tag>, RustyError> {
    let budget = get_budget(budget_id)?;
    Ok(budget.tags)
}

pub fn modify_tag(
    user_id: Uuid,
    budget_id: Uuid,
    tag_id: Uuid,
    name: Option<String>,
    periodicity: Option<Periodicity>,
    deleted: Option<bool>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).modify_tag(user_id, budget_id, tag_id, name, periodicity, deleted)
}

pub fn get_next_untagged_transaction(
    budget_id: Uuid,
) -> Result<Option<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id)?;
    Ok(budget.get_next_untagged_transaction().cloned())
}

pub fn get_transactions_for_tag(
    budget_id: Uuid,
    tag_id: Uuid,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id)?;
    let mut txs: Vec<BankTransaction> = budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|tx| tx.tag_id == Some(tag_id) && !tx.ignored)
        .cloned()
        .collect();
    txs.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(txs)
}

pub fn get_tagged_transactions(
    budget_id: Uuid,
    limit: usize,
    offset: usize,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id)?;
    let mut txs: Vec<BankTransaction> = budget
        .periods
        .iter()
        .flat_map(|p| p.transactions.iter())
        .filter(|tx| tx.tag_id.is_some() && !tx.ignored)
        .cloned()
        .collect();
    txs.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(txs.into_iter().skip(offset).take(limit).collect())
}

pub fn get_untagged_transactions(
    budget_id: Uuid,
    limit: usize,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id)?;
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

pub fn reject_transfer_pair(
    user_id: Uuid,
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
) -> Result<Uuid, RustyError> {
    with_runtime(None).reject_transfer_pair(user_id, budget_id, outgoing_tx_id, incoming_tx_id)
}

pub fn resolve_transfer_pair(
    user_id: Uuid,
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
    tag_id: Option<Uuid>,
) -> Result<Uuid, RustyError> {
    if let Some(tag_id) = tag_id {
        tag_transaction(user_id, budget_id, outgoing_tx_id, tag_id)?;
    } else {
        ignore_transaction(budget_id, user_id, outgoing_tx_id)?;
    }
    ignore_transaction(budget_id, user_id, incoming_tx_id)?;
    Ok(budget_id)
}

pub fn tag_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    tag_id: Uuid,
) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    with_runtime(None).tag_transaction(user_id, budget_id, tx_id, tag_id)?;
    tracing::info!("[perf] tag_transaction/tag_event: {:?}", t.elapsed());
    let budget = get_budget(budget_id)?;
    tracing::info!(
        "[perf] tag_transaction/load_for_rule_check: {:?}",
        t.elapsed()
    );
    let tx = budget
        .get_transaction(tx_id)
        .ok_or(RustyError::ItemNotFound(
            tx_id.to_string(),
            "Transaction not found".to_string(),
        ))?;
    let transaction_key = MatchRule::create_transaction_key(tx);
    let rule_exists = budget
        .match_rules
        .iter()
        .any(|r| r.transaction_key == transaction_key && r.tag_id == Some(tag_id));
    if !rule_exists {
        with_runtime(None).add_rule(
            user_id,
            budget_id,
            transaction_key,
            Vec::new(),
            true,
            Some(tag_id),
        )?;
        tracing::info!("[perf] tag_transaction/add_rule: {:?}", t.elapsed());
    }
    evaluate_tag_rules(user_id, budget_id)?;
    tracing::info!("[perf] tag_transaction/total: {:?}", t.elapsed());
    Ok(budget_id)
}

pub(crate) fn evaluate_tag_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    let budget = get_budget(budget_id)?;
    let matches = budget.evaluate_tag_rules();
    let match_count = matches.len();
    info!(
        "[perf] evaluate_tag_rules: {} matches found in {:?}",
        match_count,
        t.elapsed()
    );
    for (tx_id, tag_id) in matches {
        match with_runtime(None).tag_transaction(user_id, budget_id, tx_id, tag_id) {
            Ok(_) => {
                info!("Tagged a transaction, bro!");
            }
            Err(e) => error!(error = %e, "Could not tag transaction {} with tag {}", tx_id, tag_id),
        }
    }
    info!(
        "[perf] evaluate_tag_rules/total (applied {} tags): {:?}",
        match_count,
        t.elapsed()
    );
    Ok(budget_id)
}

pub fn preview_rule_matches(
    budget_id: Uuid,
    tx_id: Uuid,
) -> Result<Vec<BankTransaction>, RustyError> {
    let budget = get_budget(budget_id)?;
    Ok(budget.preview_rule_matches(tx_id))
}

pub fn modify_rule(
    user_id: Uuid,
    budget_id: Uuid,
    rule_id: Uuid,
    transaction_key: Vec<String>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).modify_rule(user_id, budget_id, rule_id, transaction_key)
}

pub fn delete_rule(user_id: Uuid, budget_id: Uuid, rule_id: Uuid) -> Result<Uuid, RustyError> {
    // Capture the rule's tag_id before deletion so we know which transactions to re-evaluate.
    let budget = get_budget(budget_id)?;
    let deleted_tag_id = budget
        .match_rules
        .iter()
        .find(|r| r.id == rule_id)
        .and_then(|r| r.tag_id);

    with_runtime(None).delete_rule(user_id, budget_id, rule_id)?;

    // If the deleted rule had a tag, re-evaluate all transactions tagged with that tag.
    // Any transaction that no longer matches ANY remaining rule for that tag gets untagged.
    if let Some(tag_id) = deleted_tag_id {
        let budget = get_budget(budget_id)?;
        let transactions_to_check: Vec<uuid::Uuid> = budget
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|tx| tx.tag_id == Some(tag_id) && !tx.ignored)
            .map(|tx| tx.id)
            .collect();

        for tx_id in transactions_to_check {
            let budget = get_budget(budget_id)?;
            if let Some(tx) = budget.get_transaction(tx_id) {
                let still_matches = budget
                    .match_rules
                    .iter()
                    .any(|r| r.tag_id == Some(tag_id) && r.matches_transaction(tx));
                if !still_matches {
                    match with_runtime(None).untag_transaction(user_id, budget_id, tx_id) {
                        Ok(_) => info!("Untagged transaction {} after rule deletion", tx_id),
                        Err(e) => {
                            error!(error = %e, "Failed to untag transaction {} after rule deletion", tx_id)
                        }
                    }
                }
            }
        }
    }

    Ok(budget_id)
}

pub fn set_item_buffer(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    buffer_target: Option<Money>,
) -> Result<Uuid, RustyError> {
    with_runtime(None).set_item_buffer(user_id, budget_id, item_id, buffer_target)
}
