use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use uuid::Uuid;
use crate::cqrs::budgets::BudgetEvent::BudgetCreated;
use crate::cqrs::framework::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetEvent {
    BudgetCreated(BudgetCreated),
    GroupAddedToBudget(GroupAdded),
    ItemAdded(ItemAdded),
    TransactionAdded(TransactionAdded),
    TransactionConnected(TransactionConnected),
    FundsReallocated(FundsReallocated),
}

// ---- Events ----

#[derive(Debug, Clone)]
pub struct BudgetCreated { pub id: Uuid, pub name: String, pub user_id: Uuid, pub default: bool, pub created_at: NaiveDateTime, updated_at: NaiveDateTime }

impl Event<Budget> for BudgetCreated {
    fn aggregate_id(&self) -> Budget::Id {
        self.id
    }

    fn apply(&self, state: &mut Budget) {
        state.id = self.id;
        state.name = self.name.clone();
        state.user_id = self.user_id;
        state.default_budget = self.default;
        state.created_at = self.created_at;
        state.updated_at = self.updated_at;
    }
}
#[derive(Debug, Clone)]
pub struct GroupAdded { pub budget_id: Uuid, pub group_id: Uuid, pub name: String }
#[derive(Debug, Clone)]
pub struct ItemAdded { pub budget_id: Uuid, pub group_id: Uuid, pub item: BudgetItem }
#[derive(Debug, Clone)]
pub struct TransactionAdded { budget_id: Uuid, tx: BankTransaction }
#[derive(Debug, Clone)]
pub struct TransactionConnected { budget_id: Uuid, tx_id: Uuid, item_id: Uuid }
#[derive(Debug, Clone)]
pub struct FundsReallocated { budget_id: Uuid, from_item: Uuid, to_item: Uuid, amount: f32 }

impl Event<Budget> for BudgetEvent {
    fn aggregate_id(&self) -> Budget::Id {
        match self {
            BudgetEvent::BudgetCreated(e) => e.id,
            BudgetEvent::GroupAddedToBudget(e) => e.budget_id,
            BudgetEvent::ItemAdded(e) => e.budget_id,
            BudgetEvent::TransactionAdded(e) => e.budget_id,
            BudgetEvent::TransactionConnected(e) => e.budget_id,
            BudgetEvent::FundsReallocated(e) => e.budget_id,
        }
    }

    fn apply(&self, state: &mut Budget) {
        match self {
            BudgetEvent::BudgetCreated(e) => e.apply(state),
            BudgetEvent::GroupAddedToBudget(e) => e.apply(state),
            BudgetEvent::ItemAdded(e) => e.apply(state),
            BudgetEvent::TransactionAdded(e) => e.apply(state),
            BudgetEvent::TransactionConnected(e) => e.apply(state),
            BudgetEvent::FundsReallocated(e) => e.apply(state),
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
        }   
    }
}

// --- Commands ---
pub struct CreateBudget { pub id: Uuid, pub name: String, pub user_id: Uuid, pub default: bool }

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

impl Command<Budget, BudgetCreated> for CreateBudget {
    fn aggregate_id(&self) -> Budget::Id {
          self.id  
    }

    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetCreated(BudgetCreated { id: Uuid::new_v4(), name: self.name, user_id: self.user_id, default: self.default, created_at: Default::default(), updated_at: Default::default()   })
    }
}

pub struct AddGroup { pub budget_id: Uuid, pub name: String }
impl Command<Budget> for AddGroup {
    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetEvent::GroupAdded { budget_id: self.budget_id, group_id: Uuid::new_v4(), name: self.name })
    }
}

pub struct AddItem { pub group_id: Uuid, pub item: BudgetItem }
impl Command<Budget> for AddItem {
    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetEvent::ItemAdded { group_id: self.group_id, item: self.item })
    }
}

pub struct AddTransaction { pub budget_id: Uuid, pub tx: BankTransaction }
impl Command<Budget> for AddTransaction {
    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetEvent::TransactionAdded { budget_id: self.budget_id, tx: self.tx })
    }
}

pub struct ConnectTransaction { pub budget_id: Uuid, pub tx_id: Uuid, pub item_id: Uuid }
impl Command<Budget> for ConnectTransaction {
    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetEvent::TransactionConnected { budget_id: self.budget_id, tx_id: self.tx_id, item_id: self.item_id })
    }
}

pub struct ReallocateFunds { pub from_item: Uuid, pub to_item: Uuid, pub amount: f32 }
impl Command<Budget> for ReallocateFunds {
    fn handle(self, _state: &Budget) -> Option<BudgetEvent> {
        Some(BudgetEvent::FundsReallocated { from_item: self.from_item, to_item: self.to_item, amount: self.amount })
    }
}

// --- Example runtime usage ---
pub fn demo() {
    let mut budget = Budget::default();

    // 1. Create budget
    let ev = CreateBudget { name: "Family Budget".into(), user_id: Uuid::new_v4(), default: true }
        .handle(&budget)
        .unwrap();
    budget.apply(&ev);

    // 2. Add group
    let ev = AddGroup { budget_id: budget.id, name: "Household".into() }.handle(&budget).unwrap();
    budget.apply(&ev);

    // 3. Add item
    let group_id = budget.budget_groups["Household"].id;
    let ev = AddItem { group_id, item: BudgetItem::new("Groceries", BudgetItemType::Expense, 500.0) }
        .handle(&budget)
        .unwrap();
    budget.apply(&ev);

    println!("Budget after setup: {:#?}", budget);
}

fn compact_demo() {
    let mut rt: Runtime<Budget, BudgetEvent> = Runtime::new();

    // happy path
    rt.execute(crate::cqrs::framework::Create(CreateAccount { id: 100, owner: "Bob".into() })).unwrap();
    rt.execute(crate::cqrs::framework::Deposit(DepositMoney { id: 100, amount_cents: 50_00 })).unwrap();
    rt.execute(crate::cqrs::framework::Withdraw(WithdrawMoney { id: 100, amount_cents: 20_00 })).unwrap();

    let acc = rt.materialize(&100).unwrap();
    println!("Account {:?}: owner={}, balance_cents={}", acc.id, acc.owner, acc.balance);

    // audit log
    println!("Events: {:?}", rt.events(&100).unwrap());
}