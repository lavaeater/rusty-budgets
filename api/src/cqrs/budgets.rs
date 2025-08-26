use std::cmp::Reverse;
use crate::cqrs::budgets::BudgetEvent::{Created, GroupAddedToBudget};
use crate::cqrs::framework::*;
use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use joydb::{Joydb, JoydbError, Model};
use joydb::adapters::JsonAdapter;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetEvent {
    Created(BudgetCreated),
    GroupAddedToBudget(GroupAdded),
    // ItemAdded(ItemAdded),
    // TransactionAdded(TransactionAdded),
    // TransactionConnected(TransactionConnected),
    // FundsReallocated(FundsReallocated),
}

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

joydb::state! {
    AppState,
    models: [StoredBudgetEvent, Budget],
}

type Db = Joydb<AppState, JsonAdapter>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCreated {
    pub budget_id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
    pub created_at: NaiveDateTime,
}

impl DomainEvent<Budget> for BudgetCreated {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.id = self.budget_id;
        state.name = self.name.clone();
        state.user_id = self.user_id;
        state.default_budget = self.default;
        state.created_at = self.created_at;
        state.updated_at = Utc::now().naive_utc();
    }
}

impl BudgetCreated {
    pub fn new(
        name: String,
        user_id: Uuid,
        default: bool,
        created_at: NaiveDateTime,
    ) -> Self {
        Self {
            budget_id: Uuid::new_v4(),
            name,
            user_id,
            default,
            created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub name: String,
}

impl DomainEvent<Budget> for GroupAdded {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn apply(&self, state: &mut Budget) {
        state.budget_groups.insert(
            self.name.clone(),
            BudgetGroup {
                id: self.group_id,
                name: self.name.clone(),
                items: Vec::default(),
            },
        );
        state.updated_at = NaiveDateTime::default();
    }
}

#[derive(Debug, Clone)]
pub struct ItemAdded {
    pub budget_id: Uuid,
    pub group_id: Uuid,
    pub item: BudgetItem,
}

#[derive(Debug, Clone)]
pub struct TransactionAdded {
    budget_id: Uuid,
    tx: BankTransaction,
}

#[derive(Debug, Clone)]
pub struct TransactionConnected {
    budget_id: Uuid,
    tx_id: Uuid,
    item_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct FundsReallocated {
    budget_id: Uuid,
    from_item: Uuid,
    to_item: Uuid,
    amount: f32,
}

impl DomainEvent<Budget> for BudgetEvent {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        match self {
            Created(e) => e.budget_id,
            GroupAddedToBudget(e) => e.budget_id,
            // BudgetEvent::ItemAdded(e) => e.budget_id,
            // BudgetEvent::TransactionAdded(e) => e.budget_id,
            // BudgetEvent::TransactionConnected(e) => e.budget_id,
            // BudgetEvent::FundsReallocated(e) => e.budget_id,
        }
    }

    fn apply(&self, state: &mut Budget) {
        match self {
            BudgetEvent::Created(e) => e.apply(state),
            BudgetEvent::GroupAddedToBudget(e) => e.apply(state),
            // BudgetEvent::ItemAdded(e) => e.apply(state),
            // BudgetEvent::TransactionAdded(e) => e.apply(state),
            // BudgetEvent::TransactionConnected(e) => e.apply(state),
            // BudgetEvent::FundsReallocated(e) => e.apply(state),
        }
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_groups: HashMap<String, BudgetGroup>,
    pub bank_transactions: Vec<BankTransaction>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub default_budget: bool,
    pub last_event: u128,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetGroup {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<BudgetItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: f32,
    pub actual_spent: f32,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetItemType {
    Income,
    Expense,
    Savings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankTransaction {
    pub id: Uuid,
    pub amount: f32,
    pub description: String,
    pub date: NaiveDate,
    pub budget_item_id: Option<Uuid>,
}

impl BudgetItem {
    pub fn new(name: &str, item_type: BudgetItemType, budgeted_amount: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            item_type,
            budgeted_amount,
            actual_spent: 0.0,
            notes: None,
            tags: Vec::new(),
        }
    }
}

impl BankTransaction {
    pub fn new(amount: f32, description: &str, date: NaiveDate) -> Self {
        Self {
            id: Uuid::new_v4(),
            amount,
            description: description.to_string(),
            date,
            budget_item_id: None,
        }
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn new(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            user_id: Uuid::new_v4(),
            default_budget: false,
            budget_groups: HashMap::new(),
            bank_transactions: Vec::new(),
            created_at: NaiveDateTime::default(),
            updated_at: NaiveDateTime::default(),
            last_event: 0,
            version: 0,
        }
    }

    fn update_timestamp(&mut self, timestamp: u128) {
        if self.last_event < timestamp {
            self.last_event = timestamp;
            self.version += 1;
        } else {
            panic!("Event timestamp is older than last event timestamp");
        }
    }

    fn version(&self) -> u64 {
        self.version
    }
}

// --- Commands ---
pub struct CreateBudget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub default: bool,
}

impl CreateBudget {
    pub fn new(id: Uuid, name: String, user_id: Uuid, default: bool) -> Self {
        Self {
            id,
            name,
            user_id,
            default,
        }
    }
}

impl Debug for CreateBudget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateBudget")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("user_id", &self.user_id)
            .field("default", &self.default)
            .finish()
    }
}

impl Command<Budget, BudgetEvent> for CreateBudget {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.id
    }

    fn handle(self, _state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        Ok(Created(BudgetCreated {
            budget_id: self.id,
            name: self.name,
            user_id: self.user_id,
            default: self.default,
            created_at: Default::default(),
        }))
    }
}

pub struct AddGroup {
    pub budget_id: Uuid,
    pub name: String,
}

impl AddGroup {
    pub fn new(budget_id: Uuid, name: String) -> Self {
        Self {
            budget_id,
            name,
        }
    }
}

impl Debug for AddGroup {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddGroup")
            .field("budget_id", &self.budget_id)
            .field("name", &self.name)
            .finish()
    }
}

impl Command<Budget, BudgetEvent> for AddGroup {
    fn aggregate_id(&self) -> <Budget as Aggregate>::Id {
        self.budget_id
    }

    fn handle(self, _state: Option<&Budget>) -> anyhow::Result<BudgetEvent> {
        Ok(GroupAddedToBudget(GroupAdded {
            budget_id: self.budget_id,
            group_id: Uuid::new_v4(),
            name: self.name,
        }))
    }
}

pub struct JoyDbBudgetRuntime {
    pub db: Db,
}

impl JoyDbBudgetRuntime {
    fn new() -> Self {
        Self { db: Db::open("data.json").unwrap() }
    }
    fn fetch_events(&self, id: &Uuid, last_timestamp: u128) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        let mut events: Vec<StoredBudgetEvent> = self
            .db
            .get_all_by(|e: &StoredBudgetEvent| e.aggregate_id == *id && e.timestamp > last_timestamp)?;
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
            },
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
            },
        }
    }

    fn snapshot(&self, agg: &Budget) -> anyhow::Result<()> {
        self.db.upsert(agg)?;
        Ok(())
    }

    fn hydrate(&mut self, id: Uuid, events: Vec<StoredBudgetEvent>) {
        todo!()
    }

    fn append(&mut self, ev: BudgetEvent) {
        let stored_event = StoredEvent::new(ev);
        self.db.insert(&stored_event).unwrap();
    }
    
    fn events(&self, id: &Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
         self
            .fetch_events(id, 0)
    }
}


// pub struct AddItem {
//     pub group_id: Uuid,
//     pub item: BudgetItem,
// }
// impl Command<Budget> for AddItem {
//     fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
//         Some(BudgetEvent::ItemAdded {
//             group_id: self.group_id,
//             item: self.item,
//         })
//     }
// }
// 
// pub struct AddTransaction {
//     pub budget_id: Uuid,
//     pub tx: BankTransaction,
// }
// impl Command<Budget> for AddTransaction {
//     fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
//         Some(BudgetEvent::TransactionAdded {
//             budget_id: self.budget_id,
//             tx: self.tx,
//         })
//     }
// }
// 
// pub struct ConnectTransaction {
//     pub budget_id: Uuid,
//     pub tx_id: Uuid,
//     pub item_id: Uuid,
// }
// impl Command<Budget> for ConnectTransaction {
//     fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
//         Some(BudgetEvent::TransactionConnected {
//             budget_id: self.budget_id,
//             tx_id: self.tx_id,
//             item_id: self.item_id,
//         })
//     }
// }
// 
// pub struct ReallocateFunds {
//     pub from_item: Uuid,
//     pub to_item: Uuid,
//     pub amount: f32,
// }
// impl Command<Budget> for ReallocateFunds {
//     fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
//         Some(BudgetEvent::FundsReallocated {
//             from_item: self.from_item,
//             to_item: self.to_item,
//             amount: self.amount,
//         })
//     }
// }

#[cfg(test)]
#[test]
pub fn testy() {
    
    let mut rt = JoyDbBudgetRuntime::new();
    let budget_id = Uuid::new_v4();

    // happy path
    rt.execute(CreateBudget::new(budget_id, "Family Budget".into(), Uuid::new_v4(), true)).unwrap();
    rt.execute(AddGroup::new(budget_id, "Salaries".into())).unwrap();
    let budget = rt.materialize(&budget_id).unwrap();
    rt.snapshot(&budget).unwrap();
    rt.execute(AddGroup::new(budget_id, "New group".into())).unwrap();
    // rt.execute(crate::cqrs::framework::Deposit(DepositMoney { id: 100, amount_cents: 50_00 })).unwrap();
    // rt.execute(crate::cqrs::framework::Withdraw(WithdrawMoney { id: 100, amount_cents: 20_00 })).unwrap();

    let budget_agg = rt.materialize(&budget_id).unwrap();
    println!("Budget {:?}: name={}, default={}", budget_agg.id, budget_agg.name, budget_agg.default_budget);

    // audit log
    println!("Events: {:?}", rt.events(&budget_id).unwrap());
}