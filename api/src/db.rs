pub const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

use crate::api_error::RustyError;
use crate::cqrs::framework::{AsyncRuntime, DomainEvent};
use crate::cqrs::runtime::{AsyncBudgetCommandsTrait, PgRuntime, create_runtime};
use crate::import::{import_from_path, import_from_skandia_excel_bytes};
use crate::models::{User, Budget, BudgetEvent, MonthBeginsOn, Currency, BudgetingType, CostKind, Matching, Money, PeriodId, Periodicity, MatchRule, Tag, BankTransaction};
use chrono::NaiveDate;
use dioxus::logger::tracing;
use dioxus::logger::tracing::error;
use dioxus::logger::tracing::info;
use std::env;
use std::path::PathBuf;
use tokio::sync::OnceCell;
use uuid::Uuid;

fn get_data_file() -> PathBuf {
    env::var("DATA_FILE").map_or_else(|_| {
            info!("DATA_FILE not set, using default data.json");
            PathBuf::from("data.json")
        }, PathBuf::from)
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
                panic!(
                    "Could not get default user ({DEFAULT_USER_EMAIL}): {e}\n\
                     The schema is applied automatically at startup, so this usually means \
                     the database is reachable but empty. Seed it by importing the JoyDB log:\n  \
                     cargo run -p api --bin api --features server\n\
                     (set DATA_FILE first if you do not want the default data.json)"
                );
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

/// # Panics
/// Panics if a rule match's transaction cannot be located in any budget period —
/// this would indicate a data inconsistency, since rule matches are derived from
/// the budget's own transactions.
pub async fn evaluate_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    for rule_match in &budget.evaluate_rules() {
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

pub async fn reallocate_funds(
    user_id: Uuid,
    budget_id: Uuid,
    period_id: PeriodId,
    from_actual_id: Uuid,
    to_actual_id: Uuid,
    amount: Money,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .reallocate_budgeted_funds(user_id, budget_id, period_id, from_actual_id, to_actual_id, amount)
        .await?;
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
    info!("Auto budgeting period {} ({} items)", period_id, budget.items.len());

    for item in &budget.items {
        let tag_ids: std::collections::HashSet<Uuid> = item.tag_ids.iter().copied().collect();
        let raw_sum: Money = period
            .transactions
            .iter()
            .filter(|tx| !tx.ignored && tx.tag_id.is_some_and(|tid| tag_ids.contains(&tid)))
            .map(|tx| tx.amount)
            .sum();
        let actual_amount = match item.budgeting_type {
            BudgetingType::Expense | BudgetingType::Savings => raw_sum.abs(),
            _ => raw_sum,
        };

        if actual_amount.is_zero() {
            continue;
        }

        let existing = period.actual_items.iter().find(|a| a.budget_item_id == item.id);
        let (actual_id, already_budgeted) = if let Some(a) = existing {
            (a.id, !a.budgeted_amount.is_zero())
        } else {
            match rt.add_actual(user_id, budget_id, item.id, Money::zero(budget.currency), period_id).await {
                Ok(id) => (id, false),
                Err(e) => {
                    error!(error = %e, "Could not create actual for item {} in period {}", item.id, period_id);
                    continue;
                }
            }
        };

        if !already_budgeted {
            match rt.modify_actual(user_id, budget_id, actual_id, period_id, Some(actual_amount), None).await {
                Ok(_) => {}
                Err(e) => error!(error = %e, "Could not set budgeted amount for actual {}", actual_id),
            }
        }
    }
    Ok(())
}

pub async fn auto_budget_all(user_id: Uuid, budget_id: Uuid) -> Result<(), RustyError> {
    let budget = runtime().await.load(budget_id).await?;
    let mut period_ids: Vec<PeriodId> = budget.periods.iter().map(|p| p.id).collect();
    period_ids.sort();
    info!("Auto budgeting all {} periods", period_ids.len());
    for period_id in period_ids {
        if let Err(e) = auto_budget_period(user_id, budget_id, period_id).await {
            error!(error = %e, "Could not auto budget period {}", period_id);
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

pub async fn classify_tag(
    user_id: Uuid,
    budget_id: Uuid,
    tag_id: Uuid,
    cost_kind: CostKind,
    matching: Matching,
) -> Result<Uuid, RustyError> {
    runtime()
        .await
        .classify_tag(user_id, budget_id, tag_id, cost_kind, matching)
        .await
}

/// Chooses the month from which category balances carry forward. `None`
/// disables carryover and restores the per-period behaviour.
pub async fn configure_carryover(
    user_id: Uuid,
    budget_id: Uuid,
    from_period: Option<PeriodId>,
) -> Result<Uuid, RustyError> {
    runtime().await.configure_carryover(user_id, budget_id, from_period).await
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

/// Tags `tx_id` with `tag_id` on `current` **in memory**, adds the implied
/// `MatchRule` if one doesn't already exist, and applies every automatic
/// rule match that falls out of that — pushing every resulting event onto
/// `events`. Returns how many rule matches were applied.
///
/// Shared by [`tag_transaction`] and [`resolve_transfer_pair`] so both can
/// wrap it in their own single `load`/`append_many`/`snapshot` instead of
/// this reloading the aggregate itself.
fn tag_transaction_in_memory(
    current: &mut Budget,
    events: &mut Vec<BudgetEvent>,
    tx_id: Uuid,
    tag_id: Uuid,
) -> Result<usize, RustyError> {
    let tag_ev = current.do_transaction_tagged(tx_id, tag_id)?;
    tag_ev.apply(current);
    events.push(tag_ev.into());

    let tx = current.get_transaction(tx_id).ok_or(RustyError::ItemNotFound(
        tx_id.to_string(),
        "Transaction not found".to_string(),
    ))?;
    let transaction_key = MatchRule::create_transaction_key(tx);
    let rule_exists = current
        .match_rules
        .iter()
        .any(|r| r.transaction_key == transaction_key && r.tag_id == Some(tag_id));
    if !rule_exists {
        let rule_ev = current.add_rule(transaction_key, Vec::new(), true, Some(tag_id))?;
        rule_ev.apply(current);
        events.push(rule_ev.into());
    }

    Ok(apply_automatic_rule_matches(current, events))
}

/// Resolves one potential-transfer pair: tags (savings) or ignores
/// (internal transfer) the outgoing leg, ignores the incoming leg, and — new
/// — learns a [`crate::models::TransferRule`] from the pair's accounts and
/// descriptions, so a future matching pair can be suggested via
/// [`Budget::suggested_transfer_resolutions`] instead of resolved by hand
/// again. Skips learning if an identical rule already exists.
///
/// All of this happens against a single in-memory load, one
/// `append_many`/`snapshot` — resolving 764 pairs by hand used to cost two
/// full reloads each (one via `tag_transaction`/`ignore_transaction`, one
/// via the other); now it's one, plus this no longer does its learning as a
/// separate step either.
pub async fn resolve_transfer_pair(
    user_id: Uuid,
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
    tag_id: Option<Uuid>,
) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();

    if let Some(tag_id) = tag_id {
        tag_transaction_in_memory(&mut current, &mut events, outgoing_tx_id, tag_id)?;
    } else {
        let ignore_ev = current.ignore_transaction(outgoing_tx_id)?;
        ignore_ev.apply(&mut current);
        events.push(ignore_ev.into());
    }

    let ignore_in_ev = current.ignore_transaction(incoming_tx_id)?;
    ignore_in_ev.apply(&mut current);
    events.push(ignore_in_ev.into());

    if let (Some(out_tx), Some(in_tx)) = (
        current.get_transaction(outgoing_tx_id),
        current.get_transaction(incoming_tx_id),
    ) {
        let outgoing_account = out_tx.account_number.clone();
        let incoming_account = in_tx.account_number.clone();
        let outgoing_key = MatchRule::create_transaction_key(out_tx);
        let incoming_key = MatchRule::create_transaction_key(in_tx);
        if let Ok(rule_ev) = current.add_transfer_rule(
            outgoing_account,
            incoming_account,
            outgoing_key,
            incoming_key,
            tag_id,
        ) {
            rule_ev.apply(&mut current);
            events.push(rule_ev.into());
        }
    }

    rt.append_many(user_id, events).await?;
    rt.snapshot(&current).await?;
    Ok(budget_id)
}

/// Resolves every potential-transfer pair that currently matches a learned
/// [`crate::models::TransferRule`] (see
/// [`Budget::suggested_transfer_resolutions`]) — the "confirm all
/// suggestions" bulk action, mirroring [`confirm_tag_suggestions`] for tags.
/// Doesn't learn any new rules; these matches already came from existing
/// ones.
pub async fn confirm_transfer_suggestions(user_id: Uuid, budget_id: Uuid) -> Result<usize, RustyError> {
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let matches = current.suggested_transfer_resolutions();

    let mut events: Vec<BudgetEvent> = Vec::new();
    let mut applied = 0;
    for (out_id, in_id, tag_id) in matches {
        let result = if let Some(tag_id) = tag_id {
            tag_transaction_in_memory(&mut current, &mut events, out_id, tag_id).map(|_| ())
        } else {
            current
                .ignore_transaction(out_id)
                .map(|ev| {
                    ev.apply(&mut current);
                    events.push(ev.into());
                })
                .map_err(RustyError::from)
        };
        if let Err(e) = result {
            error!(error = %e, "Could not resolve suggested transfer pair {}/{}", out_id, in_id);
            continue;
        }
        match current.ignore_transaction(in_id) {
            Ok(ev) => {
                ev.apply(&mut current);
                events.push(ev.into());
                applied += 1;
            }
            Err(e) => error!(error = %e, "Could not ignore incoming leg {} of suggested transfer pair", in_id),
        }
    }

    if !events.is_empty() {
        rt.append_many(user_id, events).await?;
        rt.snapshot(&current).await?;
    }
    info!("confirm_transfer_suggestions: applied {applied} suggestions");
    Ok(applied)
}

/// Applies every rule match currently found by [`Budget::evaluate_tag_rules`]
/// (i.e. `Matching::Automatic` tags) to `current` **in memory**, pushing the
/// resulting events onto `events`. Returns how many were applied.
///
/// Kept as a helper shared by [`tag_transaction`] and [`evaluate_tag_rules`]
/// so both can do their one `load`/`append_many`/`snapshot` around it instead
/// of each other, which used to mean a whole extra full reload per call. See
/// [`confirm_tag_suggestions`] for why the same shape isn't reused there.
fn apply_automatic_rule_matches(current: &mut Budget, events: &mut Vec<BudgetEvent>) -> usize {
    let matches = current.evaluate_tag_rules();
    let mut applied = 0;
    for (tx_id, tag_id) in matches {
        match current.do_transaction_tagged(tx_id, tag_id) {
            Ok(ev) => {
                ev.apply(current);
                events.push(ev.into());
                applied += 1;
            }
            Err(e) => error!(error = %e, "Could not tag transaction {} with tag {}", tx_id, tag_id),
        }
    }
    applied
}

/// Migrates any rule whose `transaction_key` was stored before punctuation
/// stripping was added to the tokenizer — e.g. `"ellos,"` from a
/// "CITY, CITY"-style address suffix — so it once again matches the
/// now-comma-free tokens real transactions produce. Idempotent: a no-op once
/// every rule is normalized.
///
/// Two rules can end up wanting the same normalized key (a stale `"ellos,"`
/// rule and a fresh `"ellos"` rule created after the tokenizer fix landed);
/// the first one seen is kept (renormalized if needed), later duplicates are
/// deleted outright rather than turned into a collision.
/// `(transaction_key, item_key, always_apply, tag_id)` — the identity a
/// [`MatchRule`] is deduplicated on, once its `transaction_key` is normalized.
type RuleKey = (Vec<String>, Vec<String>, bool, Option<Uuid>);

fn normalize_stale_rule_tokens(current: &mut Budget, events: &mut Vec<BudgetEvent>) -> usize {
    let rules: Vec<MatchRule> = current.match_rules.iter().cloned().collect();

    // Partition every rule by the key it *should* end up with, so a group of
    // more than one member is exactly the set of rules that must collapse
    // into one. Grouping up front (rather than resolving as we go) is what
    // lets duplicates be deleted before the survivor is renamed into their
    // spot — `Budget::modify_rule` removes-then-reinserts into the same
    // `HashSet<MatchRule>`, and a `HashSet::insert` onto an already-occupied
    // key is a silent no-op, so renaming into a live collision would drop
    // the rule instead of merging it.
    let mut groups: std::collections::HashMap<RuleKey, Vec<MatchRule>> = std::collections::HashMap::new();
    for rule in rules {
        let normalized_key = MatchRule::normalize_key(&rule.transaction_key);
        let final_key = (normalized_key, rule.item_key.clone(), rule.always_apply, rule.tag_id);
        groups.entry(final_key).or_default().push(rule);
    }

    let mut touched = 0;
    for (final_key, mut members) in groups {
        if members.len() == 1 && members[0].transaction_key == final_key.0 {
            continue; // already correct
        }

        // Keep whichever member is already correctly keyed, if any, so it
        // needs no rewrite; otherwise the choice is arbitrary.
        let keeper_idx = members
            .iter()
            .position(|r| r.transaction_key == final_key.0)
            .unwrap_or(0);
        let keeper = members.remove(keeper_idx);

        for dup in members {
            if let Ok(ev) = current.delete_rule(dup.id) {
                ev.apply(current);
                events.push(ev.into());
                touched += 1;
            }
        }

        if keeper.transaction_key != final_key.0
            && let Ok(ev) = current.modify_rule(keeper.id, final_key.0.clone())
        {
            ev.apply(current);
            events.push(ev.into());
            touched += 1;
        }
    }
    touched
}

pub async fn tag_transaction(
    user_id: Uuid,
    budget_id: Uuid,
    tx_id: Uuid,
    tag_id: Uuid,
) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    let rt = runtime().await;
    // One load, one snapshot for the whole operation: tag the transaction,
    // maybe add the match rule it implies, then apply every rule match that
    // falls out of that — all against the same in-memory `current`, instead
    // of each step reloading and replaying the full event stream.
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();

    let applied = tag_transaction_in_memory(&mut current, &mut events, tx_id, tag_id)?;

    rt.append_many(user_id, events).await?;
    rt.snapshot(&current).await?;
    tracing::info!(
        "[perf] tag_transaction/total (applied {} rule matches): {:?}",
        applied,
        t.elapsed()
    );
    Ok(budget_id)
}

pub async fn evaluate_tag_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let t = std::time::Instant::now();
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();
    normalize_stale_rule_tokens(&mut current, &mut events);
    let applied = apply_automatic_rule_matches(&mut current, &mut events);
    info!("[perf] evaluate_tag_rules: {} matches found in {:?}", applied, t.elapsed());
    if !events.is_empty() {
        rt.append_many(user_id, events).await?;
        rt.snapshot(&current).await?;
    }
    info!("[perf] evaluate_tag_rules/total (applied {} tags): {:?}", applied, t.elapsed());
    Ok(budget_id)
}

/// Confirms pending `Matching::Suggest` matches, optionally limited to one tag.
///
/// Applies matches directly against a single in-memory load rather than going
/// through [`tag_transaction`] per match: that wrapper also checks for a
/// missing rule and re-runs rule evaluation, neither of which is needed here
/// — a suggestion *is* a rule match, so the rule already exists, and the
/// matches are computed once up front.
///
/// Returns the number of transactions tagged.
pub async fn confirm_tag_suggestions(
    user_id: Uuid,
    budget_id: Uuid,
    tag_id: Option<Uuid>,
) -> Result<usize, RustyError> {
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let matches: Vec<(Uuid, Uuid)> = current
        .suggest_tag_rules()
        .into_iter()
        .filter(|(_, tid)| tag_id.is_none_or(|wanted| *tid == wanted))
        .collect();

    let mut events: Vec<BudgetEvent> = Vec::new();
    let mut applied = 0;
    for (tx_id, tid) in matches {
        match current.do_transaction_tagged(tx_id, tid) {
            Ok(ev) => {
                ev.apply(&mut current);
                events.push(ev.into());
                applied += 1;
            }
            Err(e) => error!(error = %e, "Could not confirm suggestion for tx {}", tx_id),
        }
    }
    if !events.is_empty() {
        rt.append_many(user_id, events).await?;
        rt.snapshot(&current).await?;
    }
    info!("confirm_tag_suggestions: applied {applied} suggestions");
    Ok(applied)
}

/// Applies **every** rule match, including those against `Matching::Suggest`
/// tags.
///
/// Distinct from [`evaluate_tag_rules`], which runs implicitly after an import
/// or a tagging action and therefore only applies bills. This is the explicit
/// "approve all matches" action, where the user has asked for the suggestions
/// to be applied too.
pub async fn apply_all_tag_rules(user_id: Uuid, budget_id: Uuid) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();
    normalize_stale_rule_tokens(&mut current, &mut events);

    let matches: Vec<_> = current
        .evaluate_tag_rules()
        .into_iter()
        .chain(current.suggest_tag_rules())
        .collect();
    info!("apply_all_tag_rules: applying {} matches", matches.len());
    for (tx_id, tag_id) in matches {
        match current.do_transaction_tagged(tx_id, tag_id) {
            Ok(ev) => {
                ev.apply(&mut current);
                events.push(ev.into());
            }
            Err(e) => error!(error = %e, "Could not tag transaction {} with tag {}", tx_id, tag_id),
        }
    }
    if !events.is_empty() {
        rt.append_many(user_id, events).await?;
        rt.snapshot(&current).await?;
    }
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

/// Deletes a rule and, if it was the only rule tagging some transactions
/// with its tag, untags them — all against a single in-memory load instead
/// of reloading the aggregate once per affected transaction (that used to
/// mean `2 + 2N` full reload-and-replay cycles for N affected transactions;
/// see the same fix applied to tagging in [`tag_transaction`]).
pub async fn delete_rule(user_id: Uuid, budget_id: Uuid, rule_id: Uuid) -> Result<Uuid, RustyError> {
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();

    let deleted_tag_id = current.match_rules.iter().find(|r| r.id == rule_id).and_then(|r| r.tag_id);
    let delete_ev = current.delete_rule(rule_id)?;
    delete_ev.apply(&mut current);
    events.push(delete_ev.into());

    if let Some(tag_id) = deleted_tag_id {
        let transactions_to_check: Vec<Uuid> = current
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|tx| tx.tag_id == Some(tag_id) && !tx.ignored)
            .map(|tx| tx.id)
            .collect();

        for tx_id in transactions_to_check {
            let still_matches = current.get_transaction(tx_id).is_some_and(|tx| {
                let tokens: std::collections::HashSet<String> =
                    MatchRule::create_transaction_key(tx).into_iter().collect();
                current
                    .match_rules
                    .iter()
                    .any(|r| r.tag_id == Some(tag_id) && r.matches_tokens(&tokens))
            });
            if !still_matches {
                match current.do_transaction_untagged(tx_id) {
                    Ok(ev) => {
                        ev.apply(&mut current);
                        events.push(ev.into());
                        info!("Untagged transaction {} after rule deletion", tx_id);
                    }
                    Err(e) => error!(error = %e, "Failed to untag transaction {} after rule deletion", tx_id),
                }
            }
        }
    }

    rt.append_many(user_id, events).await?;
    rt.snapshot(&current).await?;
    Ok(budget_id)
}

/// Serialises the budget's non-deleted tags and match rules to a JSON string,
/// for the user to save to a file and later replay onto another budget with
/// [`import_tags_and_rules`].
pub async fn export_tags_and_rules(budget_id: Uuid) -> Result<String, RustyError> {
    let rt = runtime().await;
    let budget = rt.load(budget_id).await?;
    let export = crate::rules_export::export_tags_and_rules(&budget);
    Ok(serde_json::to_string_pretty(&export)?)
}

/// Applies a JSON document produced by [`export_tags_and_rules`] onto
/// `budget_id`: creates any tag missing by name (reusing an existing one of
/// the same name otherwise) and any rule not already present, in a single
/// load/append/snapshot regardless of how many tags or rules are in the file.
pub async fn import_tags_and_rules(
    user_id: Uuid,
    budget_id: Uuid,
    json: &str,
) -> Result<crate::rules_export::ImportSummary, RustyError> {
    let export: crate::rules_export::RulesExport = serde_json::from_str(json)?;
    let rt = runtime().await;
    let mut current = rt.load(budget_id).await?;
    let mut events: Vec<BudgetEvent> = Vec::new();
    let summary = crate::rules_export::apply_rules_export(&mut current, &mut events, &export);
    if !events.is_empty() {
        rt.append_many(user_id, events).await?;
        rt.snapshot(&current).await?;
    }
    info!(
        "import_tags_and_rules: {} tags created, {} reused, {} rules created, {} skipped, \
         {} transfer rules created, {} skipped",
        summary.tags_created,
        summary.tags_reused,
        summary.rules_created,
        summary.rules_skipped,
        summary.transfer_rules_created,
        summary.transfer_rules_skipped
    );
    Ok(summary)
}

/// Dumps an entire budget — accounts, tags, match rules, transfer rules,
/// items, periods, and every transaction — as a JSON string, for the user
/// to save to a file and later restore with [`import_budget`] on this or
/// any other instance of the app.
pub async fn export_budget(budget_id: Uuid) -> Result<String, RustyError> {
    let budget = get_budget(budget_id).await?;
    Ok(serde_json::to_string_pretty(&budget)?)
}

/// Restores a JSON dump produced by [`export_budget`] as a brand new budget
/// for `user_id` — never merged into an existing budget. Becomes the user's
/// default budget only if they don't already have one (so importing into an
/// instance that already has an active budget can't silently swap it out).
pub async fn import_budget(user_id: Uuid, json: &str) -> Result<Uuid, RustyError> {
    let parsed: Budget = serde_json::from_str(json)?;
    let budget = crate::budget_export::prepare_imported_budget(parsed, user_id);
    let rt = runtime().await;
    let make_default = rt.get_default_budget(user_id).await.is_err();
    rt.snapshot(&budget).await?;
    rt.add_budget_to_user(user_id, budget.id, make_default).await?;
    info!(
        "import_budget: created budget {} for user {} (default: {})",
        budget.id, user_id, make_default
    );
    Ok(budget.id)
}

pub async fn set_item_buffer(
    user_id: Uuid,
    budget_id: Uuid,
    item_id: Uuid,
    buffer_target: Option<Money>,
) -> Result<Uuid, RustyError> {
    runtime().await.set_item_buffer(user_id, budget_id, item_id, buffer_target).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cqrs::framework::Runtime;
    use crate::cqrs::runtime::{BudgetCommandsTrait, JoyDbBudgetRuntime};

    #[test]
    fn normalize_stale_rule_tokens_fixes_stray_comma_and_merges_duplicate() {
        let rt = JoyDbBudgetRuntime::new_in_memory();
        let user_id = Uuid::new_v4();
        let budget_id = rt
            .create_budget(user_id, "Test Budget", true, MonthBeginsOn::default(), Currency::SEK)
            .unwrap();
        let tag_id = rt
            .create_tag(user_id, budget_id, "Ellos".to_string(), Periodicity::Monthly)
            .unwrap();

        // A stale pre-fix rule, plus a fresh post-fix rule created for the
        // same payee — exactly what happens once the tokenizer stops
        // producing "ellos," but the old rule is still stored with it.
        rt.add_rule(user_id, budget_id, vec!["ellos,".to_string()], Vec::new(), true, Some(tag_id))
            .unwrap();
        rt.add_rule(user_id, budget_id, vec!["ellos".to_string()], Vec::new(), true, Some(tag_id))
            .unwrap();

        let mut current = rt.load(budget_id).unwrap();
        assert_eq!(current.match_rules.len(), 2);

        let mut events: Vec<BudgetEvent> = Vec::new();
        let touched = normalize_stale_rule_tokens(&mut current, &mut events);
        assert!(touched >= 1);
        assert!(!events.is_empty());

        assert_eq!(current.match_rules.len(), 1, "duplicate must be merged away");
        let survivor = current.match_rules.iter().next().unwrap();
        assert_eq!(survivor.transaction_key, vec!["ellos".to_string()]);
        assert_eq!(survivor.tag_id, Some(tag_id));

        // Idempotent: running it again against the now-clean state is a no-op.
        let mut events2 = Vec::new();
        let touched2 = normalize_stale_rule_tokens(&mut current, &mut events2);
        assert_eq!(touched2, 0);
        assert!(events2.is_empty());
    }
}
