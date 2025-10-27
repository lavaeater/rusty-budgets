use crate::cqrs::framework::{Runtime, StoredEvent};
use crate::models::*;
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use joydb::adapters::{FromPath, JsonAdapter};
use joydb::{Joydb, JoydbConfig, JoydbError, JoydbMode, SyncPolicy};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

impl JoyDbBudgetRuntime {
    pub fn create_budget(
        &self,
        budget_name: &str,
        default_budget: bool,
        currency: Currency,
        user_id: Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(&user_id, &Uuid::default(), |budget| {
            budget.create_budget(budget_name.to_string(), user_id, default_budget, currency)
        })
    }

    pub fn add_item(
        &self,
        budget_id: &Uuid,
        item_name: &str,
        item_type: &BudgetingType,
        amount: &Money,
        user_id: &Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_item(item_name.to_string(), *item_type, *amount)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modify_item(
        &self,
        budget_id: &Uuid,
        item_id: &Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        budgeted_amount: Option<Money>,
        notes: Option<String>,
        tags: Option<Vec<String>>,
        user_id: &Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(user_id, budget_id,|budget| {
            budget.modify_item(*item_id, name, item_type, budgeted_amount, notes, tags)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_and_connect_tx(
        &self,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
        item_id: Uuid,
        user_id: Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        if let Ok((_, tx_id)) = self.add_transaction(
            budget_id,
            bank_account_number,
            amount,
            balance,
            description,
            date,
            user_id,
        ) {
            self.connect_transaction(&budget_id, &tx_id, &item_id, &user_id)
        } else {
            Err(anyhow::anyhow!("Failed to add transaction"))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_transaction(
        &self,
        budget_id: Uuid,
        bank_account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
        user_id: Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(&user_id, &budget_id, |budget| {
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
        budget_id: &Uuid,
        tx_id: &Uuid,
        item_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(user_id, budget_id, |budget| {
            budget.connect_transaction(*tx_id, *item_id)
        })
    }

    pub fn ignore_transaction(
        &self,
        budget_id: &Uuid,
        tx_id: &Uuid,
        user_id: &Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(user_id, budget_id, |budget| {
            budget.ignore_transaction(*tx_id)
        })
    }

    pub fn reallocate_item_funds(
        &self,
        budget_id: Uuid,
        from_item_id: Uuid,
        to_item_id: Uuid,
        amount: Money,
        user_id: Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(&user_id, &budget_id, |budget| {
            budget.reallocate_item_funds(from_item_id, to_item_id, amount)
        })
    }

    pub fn adjust_item_funds(
        &self,
        budget_id: Uuid,
        item_id: Uuid,
        amount: Money,
        user_id: Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(&user_id, &budget_id, |budget| {
            budget.adjust_item_funds(item_id, amount)
        })
    }

    pub fn add_rule(
        &self,
        budget_id: &Uuid,
        transaction_key: Vec<String>,
        item_name: String,
        always_apply: bool,
        user_id: &Uuid,
    ) -> anyhow::Result<(Budget, Uuid)> {
        self.cmd(user_id, budget_id, |budget| {
            budget.add_rule(transaction_key, item_name, always_apply)
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
                sync_policy: SyncPolicy::Periodic(Duration::from_secs(5)),
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
    pub fn cmd<F, E>(&self, user_id: &Uuid, id: &Uuid, command: F) -> anyhow::Result<(Budget, Uuid)>
    where
        F: FnOnce(&Budget) -> Result<E, crate::cqrs::framework::CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(user_id, id, |aggregate| {
            command(aggregate).map(|event| event.into())
        })
    }

    fn fetch_events(
        &self,
        id: &Uuid,
        last_timestamp: i64,
    ) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        let mut events: Vec<StoredBudgetEvent> = self.db.get_all_by(|e: &StoredBudgetEvent| {
            e.aggregate_id == *id && e.timestamp > last_timestamp
        })?;
        events.sort_by_key(|e| e.timestamp);
        Ok(events)
    }

    fn get_budget(&self, id: &Uuid) -> anyhow::Result<Option<Budget>> {
        let budget = self.db.get::<Budget>(id)?;
        if let Some(budget) = budget {
            Ok(Some(budget))
        } else {
            Ok(None)
        }
    }
}

impl Runtime<Budget, BudgetEvent> for JoyDbBudgetRuntime {
    fn load(&self, id: &Uuid) -> Result<Option<Budget>, anyhow::Error> {
        let budget: Result<Option<Budget>, anyhow::Error> = match self.db.get::<Budget>(id) {
            Err(err) => {
                match err {
                    JoydbError::Deserialize(_) => Ok(None),
                    _ => Err(err.into()),
                }
            },
            Ok(budget) => Ok(budget),
        };
        
        let budget = budget?;
        let mut  budget = budget.unwrap_or(Budget::new(*id));
        let version = budget.version;
        let events = self.fetch_events(id, budget.last_event)?;
        for ev in events {
            ev.apply(&mut budget);
        }
        let version = budget.version - version;
        if version > 10 { // more than 10 events since last snapshot
            tracing::info!("More than 10 events since last snapshot, snapshotting");
            self.snapshot(&budget)?;
        }
        Ok(Some(budget))
    }

    fn snapshot(&self, agg: &Budget) -> anyhow::Result<()> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn append(&self, user_id: &Uuid, ev: BudgetEvent) -> anyhow::Result<()> {
        let stored_event = StoredEvent::new(ev, *user_id);
        self.db.insert(&stored_event)?;
        Ok(())
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}
