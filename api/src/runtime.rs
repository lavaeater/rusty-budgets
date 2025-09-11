#[cfg(feature = "server")]
use std::str::FromStr;
#[cfg(feature = "server")]
use joydb::Joydb;
#[cfg(feature = "server")]
use dioxus::logger::tracing;
#[cfg(feature = "server")]
use joydb::adapters::JsonAdapter;

use joydb::{Model};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use crate::cqrs::budget::Budget;
#[cfg(feature = "server")]
use crate::cqrs::budgets::BudgetEvent;
#[cfg(feature = "server")]
use crate::cqrs::framework::{Runtime, StoredEvent};
#[cfg(feature = "server")]
use crate::models::User;

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct UserBudgets {
    pub id: Uuid,
    pub budgets: Vec<(Uuid, bool)>,}

#[cfg(feature = "server")]
joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget, User, UserBudgets],
}

#[cfg(feature = "server")]
type StoredBudgetEvent = StoredEvent<Budget, BudgetEvent>;

#[cfg(feature = "server")]
impl Model for StoredBudgetEvent {
    type Id = Uuid;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn model_name() -> &'static str {
        "budget_event"
    }
}

#[cfg(feature = "server")]
pub type Db = Joydb<AppState, JsonAdapter>;

#[cfg(feature = "server")]
pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

#[cfg(feature = "server")]
impl JoyDbBudgetRuntime {
    pub fn new() -> Self {
        Self {
            db: Db::open("data.json").unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    pub fn cmd<F, E>(&self, id: Uuid, command: F) -> anyhow::Result<Budget>
    where
        F: FnOnce(&Budget) -> Result<E, crate::cqrs::framework::CommandError>,
        E: Into<BudgetEvent>,
    {
        self.execute(id, |aggregate| {
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

#[cfg(feature = "server")]
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

    fn append(&self, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev);
        self.db.insert(&stored_event).unwrap();
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
#[test]
pub fn testy() -> anyhow::Result<()> {
    let mut rt = JoyDbBudgetRuntime::new();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let e = rt.cmd(budget_id, |budget| budget.create_budget("Test Budget".to_string(), user_id, true))?;
    assert_eq!(e.name, "Test Budget");
    assert!(e.default_budget);
    assert_eq!(e.budget_groups.values().len(), 0);
    let res = rt.cmd(budget_id, |budget| budget.add_group(Uuid::new_v4(), "Inkomster".to_string()));
    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.budget_groups.values().len(), 1);
    let e = rt.cmd(budget_id, |budget| budget.add_group(Uuid::new_v4(), "Inkomster".to_string())).err();
    assert!(e.is_some());
    assert_eq!(e.unwrap().to_string(), "Validation error: Budget group already exists");
    
    let e = rt.cmd(budget_id, |budget| budget.add_group(Uuid::new_v4(), "Utgifter".to_string()))?;
    assert_eq!(e.budget_groups.values().len(), 2);

    let budget_agg = rt.materialize(&budget_id)?;
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );

    for group in budget_agg.budget_groups.values() {
        println!("Group: {}", group.name);
    }

    // audit log
    println!("Events: {:?}", rt.events(&budget_id)?);
    let _ = rt.db.delete_all_by(|b: &Budget| b.id == budget_id);
    let _ = rt.db.delete_all_by(|b: &StoredBudgetEvent| b.aggregate_id == budget_id);
    let _ = rt.db.delete_all_by(|b: &UserBudgets| b.id == user_id);
    Ok(())
}
