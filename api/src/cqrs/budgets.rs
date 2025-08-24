use chrono::{NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// --- Import the minimal CQRS core ---
pub trait Aggregate: Default + Clone {
    fn apply(&mut self, event: &Event);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    BudgetCreated { id: Uuid, name: String, user_id: Uuid, default: bool },
    GroupAdded { budget_id: Uuid, group_id: Uuid, name: String },
    ItemAdded { group_id: Uuid, item: BudgetItem },
    TransactionAdded { budget_id: Uuid, tx: BankTransaction },
    TransactionConnected { budget_id: Uuid, tx_id: Uuid, item_id: Uuid },
    FundsReallocated { from_item: Uuid, to_item: Uuid, amount: f32 },
}

pub trait Command<A: Aggregate> {
    fn handle(self, state: &A) -> Option<Event>;
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_groups: HashMap<String, BudgetGroup>,
    pub bank_transactions: Vec<BankTransaction>,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
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
    fn apply(&mut self, event: &Event) {
        match event {
            Event::BudgetCreated { id, name, user_id, default } => {
                self.id = *id;
                self.name = name.clone();
                self.user_id = *user_id;
                self.default_budget = *default;
                self.created_at = Some(Utc::now().naive_utc());
                self.updated_at = Some(Utc::now().naive_utc());
            }
            Event::GroupAdded { group_id, name, .. } => {
                self.budget_groups.insert(
                    name.clone(),
                    BudgetGroup { id: *group_id, name: name.clone(), items: Vec::new() },
                );
            }
            Event::ItemAdded { group_id, item } => {
                if let Some(group) = self.budget_groups.values_mut().find(|g| g.id == *group_id) {
                    group.items.push(item.clone());
                }
            }
            Event::TransactionAdded { tx, .. } => {
                self.bank_transactions.push(tx.clone());
            }
            Event::TransactionConnected { tx_id, item_id, .. } => {
                if let Some(tx) = self.bank_transactions.iter_mut().find(|t| t.id == *tx_id) {
                    tx.budget_item_id = Some(*item_id);
                }
                if let Some(item) = self.budget_groups.values_mut().flat_map(|g| &mut g.items).find(|i| i.id == *item_id) {
                    item.actual_spent = self.bank_transactions.iter().filter(|t| t.budget_item_id == Some(*item_id)).map(|t| t.amount).sum();
                }
            }
            Event::FundsReallocated { from_item, to_item, amount } => {
                if let Some(from) = self.budget_groups.values_mut().flat_map(|g| &mut g.items).find(|i| i.id == *from_item) {
                    from.budgeted_amount -= amount;
                }
                if let Some(to) = self.budget_groups.values_mut().flat_map(|g| &mut g.items).find(|i| i.id == *to_item) {
                    to.budgeted_amount += amount;
                }
            }
        }
        self.updated_at = Some(Utc::now().naive_utc());
    }
}

// --- Commands ---
pub struct CreateBudget { pub name: String, pub user_id: Uuid, pub default: bool }
impl Command<Budget> for CreateBudget {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::BudgetCreated { id: Uuid::new_v4(), name: self.name, user_id: self.user_id, default: self.default })
    }
}

pub struct AddGroup { pub budget_id: Uuid, pub name: String }
impl Command<Budget> for AddGroup {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::GroupAdded { budget_id: self.budget_id, group_id: Uuid::new_v4(), name: self.name })
    }
}

pub struct AddItem { pub group_id: Uuid, pub item: BudgetItem }
impl Command<Budget> for AddItem {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::ItemAdded { group_id: self.group_id, item: self.item })
    }
}

pub struct AddTransaction { pub budget_id: Uuid, pub tx: BankTransaction }
impl Command<Budget> for AddTransaction {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::TransactionAdded { budget_id: self.budget_id, tx: self.tx })
    }
}

pub struct ConnectTransaction { pub budget_id: Uuid, pub tx_id: Uuid, pub item_id: Uuid }
impl Command<Budget> for ConnectTransaction {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::TransactionConnected { budget_id: self.budget_id, tx_id: self.tx_id, item_id: self.item_id })
    }
}

pub struct ReallocateFunds { pub from_item: Uuid, pub to_item: Uuid, pub amount: f32 }
impl Command<Budget> for ReallocateFunds {
    fn handle(self, _state: &Budget) -> Option<Event> {
        Some(Event::FundsReallocated { from_item: self.from_item, to_item: self.to_item, amount: self.amount })
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
