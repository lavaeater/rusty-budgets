use std::str::FromStr;
#[cfg(feature = "server")]
use joydb::Joydb;
#[cfg(feature = "server")]
use dioxus::logger::tracing;
#[cfg(feature = "server")]
use joydb::adapters::JsonAdapter;

use joydb::{Model};
use uuid::Uuid;
use crate::cqrs::budget::Budget;
use crate::cqrs::budgets::BudgetEvent;
use crate::cqrs::framework::{Runtime, StoredEvent};
use crate::models::User;
// impl Display for BudgetEvent {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         match self {
//             BudgetEvent::BudgetCreated(event) => write!(f, "Budget created: {}", event.budget_id),
//             // BudgetEvent::GroupAdded(event) => {
//             //     write!(f, "Group added to budget: {}", event.group_id)
//             // }
//             // BudgetEvent::ItemAdded(event) => write!(f, "Item added: {}", event.item.id),
//             // BudgetEvent::TransactionAdded(event) => {
//             //     write!(f, "Transaction added: {}", event.tx)
//             // }
//             // BudgetEvent::TransactionConnected(event) => {
//             //     write!(f, "Transaction connected: {}", event.tx_id)
//             // }
//             // BudgetEvent::FundsReallocated(event) => {
//             //     write!(f, "Funds reallocated: {}", event.from_item)
//             // }
//         }
//     }
// }
//

#[cfg(feature = "server")]
joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget, User],
}

#[cfg(feature = "server")]
type StoredBudgetEvent = StoredEvent<Budget, BudgetEvent>;

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

impl JoyDbBudgetRuntime {
    pub fn new() -> Self {
        Self {
            db: Db::open("data.json").unwrap(),
        }
    }

    /// Ergonomic command execution - eliminates all the boilerplate!
    /// Usage: rt.cmd(id, |budget| budget.create_budget(name, user_id, default))
    pub fn cmd<F, E>(&mut self, id: Uuid, command: F) -> anyhow::Result<BudgetEvent>
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

    fn append(&mut self, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev);
        self.db.insert(&stored_event).unwrap();
    }

    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        self.fetch_events(id, 0)
    }
}
#[cfg(test)]
#[test]
pub fn testy() -> anyhow::Result<()> {
    let mut rt = JoyDbBudgetRuntime::new();
    let budget_id = Uuid::from_str("760365f3-fa77-49a8-aa77-4717748e52ae")?;
    let user_id = Uuid::new_v4();

    // rt.cmd(budget_id, |budget| budget.create_budget("Test Budget".to_string(), user_id, true))?;
    // rt.cmd(budget_id, |budget| budget.add_group(Uuid::new_v4(), "Inkomster".to_string()))?;

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
    Ok(())
}
