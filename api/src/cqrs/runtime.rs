use std::path::Path;
use joydb::Joydb;
use joydb::adapters::JsonAdapter;

use joydb::{Model};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::cqrs::budget::Budget;
use crate::cqrs::budgets::BudgetEvent;
use crate::cqrs::framework::{Runtime, StoredEvent};
use crate::models::User;

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct UserBudgets {
    pub id: Uuid,
    pub budgets: Vec<(Uuid, bool)>,}

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
        Self {
            db: Db::open(path).unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    pub fn cmd<F, E>(&self, user_id: &Uuid, id: &Uuid, command: F) -> anyhow::Result<Budget>
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
        let budget = self.get_budget(id)?;
        match budget {
            Some(mut budget) => {
                let events = self.fetch_events(id, budget.last_event)?;
                for ev in events {
                    ev.apply(&mut budget);
                }
                Ok(Some(budget))
            }
            None => {
                let mut budget = Budget {
                    id: *id,
                    ..Default::default()
                };
                let events = self.fetch_events(id, budget.last_event)?;
                for ev in events {
                    ev.apply(&mut budget);
                }
                Ok(Some(budget))
            }
        }
    }

    fn snapshot(&self, agg: &Budget) -> anyhow::Result<()> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn append(&self, user_id: &Uuid, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev, *user_id);
        self.db.insert(&stored_event).unwrap();
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}
