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

#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct AggregateModel {
    pub id: Uuid,
}

joydb::state! {
    AppState,
    models: [StoredBudgetEvent, AggregateModel],
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

    fn handle(self, _state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
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

    fn handle(self, _state: Option<&Budget>) -> Result<BudgetEvent, CommandError> {
        Ok(GroupAddedToBudget(GroupAdded {
            budget_id: self.budget_id,
            group_id: Uuid::new_v4(),
            name: self.name,
        }))
    }
}

pub struct InMemoryRuntime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    streams: HashMap<A::Id, Vec<StoredEvent<A, E>>>,
    on_event: Option<Box<dyn FnMut(&A::Id, StoredEvent<A, E>)>>,
}

impl<A, E> InMemoryRuntime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    pub fn new() -> Self {
        Self { streams: HashMap::new(), on_event: None }
    }

    pub fn with_storage_function(on_event: Box<dyn FnMut(&A::Id, StoredEvent<A, E>)>) -> Self {
        Self { streams: HashMap::new(), on_event: Some(on_event) }
    }
}

impl<A, E> Runtime<A, E> for InMemoryRuntime<A, E>
where
    A: Aggregate,
    E: DomainEvent<A>,
{
    fn load(&self, id: &A::Id) -> Option<A> {
        self.streams.get(id).map(|events| {
            let mut state = A::new(id.clone());
            for ev in events {
                ev.data.apply(&mut state);
            }
            state
        })
    }

    fn hydrate(&mut self, id: A::Id, events: Vec<StoredEvent<A, E>>) {
        self.streams.insert(id, events);
    }

    fn append(&mut self, ev: E) {
        let aggregate_id = ev.aggregate_id();
        let stored_event = StoredEvent::new(ev);
        if let Some(ref mut on_event) = self.on_event {
            on_event(&aggregate_id, stored_event.clone());
        }
        self.streams.entry(aggregate_id).or_default().push(stored_event);
    }

    fn events(&self, id: &A::Id) -> Option<Vec<StoredEvent<A, E>>> {
        self.streams.get(id).map(|v| v.clone())
    }
}

pub struct JoyDbBudgetRuntime;

impl JoyDbBudgetRuntime {
    fn fetch_events(&self, id: Uuid) -> anyhow::Result<Vec<StoredBudgetEvent>> {
        let db = Db::open("data.json")?;
        let mut events: Vec<StoredBudgetEvent> =  db.get_all_by(|e: &StoredBudgetEvent| e.aggregate_id == id)?;
        events.sort_by_key(|e| Reverse(e.timestamp));
        Ok(events)
    }
}

impl Runtime<Budget, BudgetEvent> for JoyDbBudgetRuntime {
    fn load(&self, id: &Uuid) -> Option<Budget> {
        let mut state = Budget::new(id.clone());
        for ev in events {
            ev.data.apply(&mut state);
        }
        Some(state)
    }

    fn hydrate(&mut self, id: BudgetId, events: Vec<StoredBudgetEvent>) {
        self.conn.insert_events(id, events);
    }

    fn append(&mut self, ev: BudgetEvent) {
        let stored = StoredEvent::new(ev);
        self.conn.insert_event(stored);
    }

    fn events(&self, id: &BudgetId) -> Option<Vec<StoredBudgetEvent>> {
        Some(self.conn.fetch_events(id))
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
    let db =  Db::open("data.json").unwrap();
    let db_clone = db.clone();
    let mut rt: Borgtime<Budget, BudgetEvent> = Borgtime::with_storage_function(Box::new(move |aggregate_id: &Uuid, event| { 
        let am = AggregateModel {
            id: *aggregate_id,
        };
        db_clone.upsert(&am).unwrap();
        db_clone.insert(&event).unwrap();
    }));
    
    if let Ok(mut aggregates) = db.get_all::<AggregateModel>() {
        for agg in aggregates {
            if let Ok(mut events) = db.get_all_by(|e: &StoredBudgetEvent| e.aggregate_id == agg.id) {
                rt.hydrate(agg.id, events);
            }
        }
    }
    
    let budget_id = Uuid::new_v4();

    // happy path
    rt.execute(CreateBudget::new(budget_id, "Family Budget".into(), Uuid::new_v4(), true)).unwrap();
    // rt.execute(crate::cqrs::framework::Deposit(DepositMoney { id: 100, amount_cents: 50_00 })).unwrap();
    // rt.execute(crate::cqrs::framework::Withdraw(WithdrawMoney { id: 100, amount_cents: 20_00 })).unwrap();

    let budget_agg = rt.materialize(&budget_id).unwrap();
    println!("Budget {:?}: name={}, default={}", budget_agg.id, budget_agg.name, budget_agg.default_budget);

    // audit log
    println!("Events: {:?}", rt.events(&budget_id).unwrap());
}