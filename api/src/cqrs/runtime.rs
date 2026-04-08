use crate::cqrs::framework::{CommandError, Runtime, StoredEvent};
use crate::models::*;
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use joydb::adapters::{FromPath, JsonAdapter};
use joydb::{Joydb, JoydbConfig, JoydbMode, SyncPolicy};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;
use crate::api_error::RustyError;

impl JoyDbBudgetRuntime {
    pub fn create_budget(
        &self,
        user_id: Uuid,
        budget_name: &str,
        default_budget: bool,
        month_begins_on: MonthBeginsOn,
        currency: Currency,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, Uuid::default(), |budget| {
            budget.create_budget(budget_name.to_string(), user_id, month_begins_on, default_budget, currency)
        })
    }

    pub fn add_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_name: String,
        item_type: BudgetingType,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_item(item_name.to_string(), item_type)
        })
    }

    pub fn add_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        period_id: PeriodId,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_actual(item_id, period_id, amount)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modify_item(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        tag_ids: Option<Vec<Uuid>>,
        periodicity: Option<Periodicity>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_item(item_id, name, item_type, tag_ids, periodicity)
        })
    }

    pub fn create_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        name: String,
        periodicity: Periodicity,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_tag(name, periodicity)
        })
    }

    pub fn modify_tag(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tag_id: Uuid,
        name: Option<String>,
        periodicity: Option<Periodicity>,
        deleted: Option<bool>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_tag(tag_id, name, periodicity, deleted)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modify_actual(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id,|budget| {
            budget.modify_actual(actual_id, period_id, budgeted_amount, actual_amount, None, None)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_and_connect_tx(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>
    ) -> Result<Uuid, RustyError> {
        let tx_id = self.add_transaction(
            user_id,
            budget_id,
            bank_account_number,
            amount,
            balance,
            description,
            date,
        )?;
        self.connect_transaction(user_id, budget_id, tx_id, actual_id)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_transaction(
                bank_account_number.to_string(),
                amount,
                balance,
                description.to_string(),
                date,
            )
        })
    }

    pub fn connect_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        let (amount, existing_allocations) = {
            let budget = self.load(budget_id)?;
            let amount = budget
                .get_transaction(tx_id)
                .map(|tx| tx.amount)
                .ok_or_else(|| RustyError::ItemNotFound(tx_id.to_string(), "Transaction not found".to_string()))?;
            let existing = budget
                .allocations_for_transaction(tx_id)
                .iter()
                .map(|a| (a.id, a.transaction_id))
                .collect::<Vec<_>>();
            (amount, existing)
        };
        for (alloc_id, transaction_id) in existing_allocations {
            self.delete_allocation(user_id, budget_id, alloc_id, transaction_id)?;
        }
        self.create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new())
    }

    pub fn ensure_account(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        account_number: &str,
        description: &str,
    ) -> Result<Uuid, RustyError> {
        let budget = self.load(budget_id)?;
        if let Some(existing) = budget.get_account(account_number) {
            return Ok(existing.id);
        }
        self.cmd(user_id, budget_id, |budget| {
            budget.create_bank_account(
                account_number.to_string(),
                description.to_string(),
            )
        })
    }

    pub fn ignore_transaction(
        &self,
        budget_id: Uuid,
        tx_id: Uuid,
        user_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.ignore_transaction(tx_id)
        })
    }

    pub fn reallocate_budgeted_funds(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        period_id: PeriodId,
        from_actual_id: Uuid,
        to_actual_id: Uuid,
        amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reallocate_budgeted_funds(period_id, from_actual_id, to_actual_id, amount)
        })
    }

    pub fn adjust_budgeted_amount(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Money,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.adjust_actual_budgeted_funds(actual_id, period_id, budgeted_amount)
        })
    }

    pub fn add_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_key: Vec<String>,
        item_key: Vec<String>,
        always_apply: bool,
        tag_id: Option<Uuid>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_rule(transaction_key, item_key, always_apply, tag_id)
        })
    }

    pub fn tag_transaction(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.do_transaction_tagged(tx_id, tag_id)
        })
    }

    pub fn reject_transfer_pair(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.reject_transfer_pair(outgoing_tx_id, incoming_tx_id)
        })
    }

    pub fn modify_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
        transaction_key: Vec<String>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.modify_rule(rule_id, transaction_key)
        })
    }

    pub fn delete_rule(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        rule_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| budget.delete_rule(rule_id))
    }

    pub fn set_item_buffer(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        item_id: Uuid,
        buffer_target: Option<Money>,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.set_item_buffer(item_id, buffer_target)
        })
    }

    pub fn create_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        transaction_id: Uuid,
        actual_id: Uuid,
        amount: Money,
        tag: String,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.create_allocation(transaction_id, actual_id, amount, tag)
        })
    }

    pub fn delete_allocation(
        &self,
        user_id: Uuid,
        budget_id: Uuid,
        allocation_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Uuid, RustyError> {
        self.cmd(user_id, budget_id, |budget| {
            budget.delete_allocation(allocation_id, transaction_id)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct UserBudgets {
    pub id: Uuid,
    pub budgets: Vec<(Uuid, bool)>,
}

joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget, User, UserBudgets],
}

pub type StoredBudgetEvent = StoredEvent<Budget, BudgetEvent>;

impl Model for StoredBudgetEvent {
    type Id = Uuid;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn model_name() -> &'static str {
        "budget_event"
    }
}

pub type Db = Joydb<AppState, JsonAdapter>;

pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

impl JoyDbBudgetRuntime {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let adapter = JsonAdapter::from_path(path);
        let config = JoydbConfig {
            mode: JoydbMode::Persistent {
                adapter,
                sync_policy: SyncPolicy::Periodic(Duration::from_secs(30)),
            },
        };
        Self {
            db: Db::open_with_config(config).unwrap(),
        }
    }

    pub fn new_in_memory() -> Self {
        Self {
            db: Db::new_in_memory().unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    pub fn cmd<F, E>(&self, user_id: Uuid, id: Uuid, command: F) -> Result<Uuid, RustyError>
    where
        F: FnOnce(&Budget) -> Result<E, CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(|event| event.into())
        })
    }

    fn fetch_events(
        &self,
        id: Uuid,
        last_timestamp: i64,
    ) -> Result<Vec<StoredBudgetEvent>, RustyError> {
        let mut events: Vec<StoredBudgetEvent> = self.db.get_all_by(|e: &StoredBudgetEvent| {
            e.aggregate_id == id && e.timestamp > last_timestamp
        })?;
        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }

    fn get_budget(&self, id: Uuid) -> Result<Option<Budget>, RustyError> {
        let budget = self.db.get::<Budget>(&id)?;
        Ok(budget)
    }

    pub fn undo_last(&self, budget_id: Uuid) -> Result<bool, RustyError> {
        let mut events = self.events(budget_id)?;
        if events.is_empty() {
            return Ok(false);
        }
        events.sort_by_key(|e| e.timestamp);
        let last_event_id = events.last().unwrap().id;
        self.db.delete::<StoredBudgetEvent>(&last_event_id)?;
        Ok(true)
    }
}

impl Runtime<Budget, BudgetEvent> for JoyDbBudgetRuntime {
    fn load(&self, id: Uuid) -> Result<Budget, RustyError> {
        let t = std::time::Instant::now();
        let budget = self.db.get::<Budget>(&id)?;

        tracing::debug!("Loaded budget is some: {}", budget.is_some());
        let mut budget = budget.unwrap_or(Budget::new(id));
        let version = budget.version;
        tracing::debug!("Loaded budget has version {} and last event at {}", version, budget.last_event);
        let events = self.fetch_events(id, budget.last_event)?;
        let event_count = events.len();
        for ev in events {
            ev.apply(&mut budget);
        }
        tracing::info!(
            "[perf] load: replayed {} events in {:?}",
            event_count, t.elapsed()
        );
        if event_count > 0 {
            self.snapshot(&budget)?;
        }
        Ok(budget)
    }

    fn snapshot(&self, agg: &Budget) -> Result<(), RustyError> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn append(&self, user_id: Uuid, ev: BudgetEvent) -> Result<(), RustyError> {
        let stored_event = StoredEvent::new(ev, user_id);
        self.db.insert(&stored_event)?;
        Ok(())
    }

    fn events(&self, id: Uuid) -> Result<Vec<StoredBudgetEvent>, RustyError> {
        self.fetch_events(id, 0)
    }
}
