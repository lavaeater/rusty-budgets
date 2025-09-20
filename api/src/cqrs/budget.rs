use crate::cqrs::framework::DomainEvent;
use crate::cqrs::domain_events::{BudgetCreated, GroupAdded, ItemAdded, TransactionAdded, TransactionConnected};
use crate::cqrs::framework::Aggregate;
use crate::cqrs::money::Money;
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        GroupAdded,
        ItemAdded,
        TransactionAdded,
        TransactionConnected,
        // FundsReallocated
        // ... add other events here
    }
}


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Store {
    all: HashSet<u64>,       // uniqueness check
    by_id: HashMap<Uuid, BankTransaction> // fast lookup
}

impl Store {
    fn new() -> Self {
        Self {
            all: HashSet::new(),
            by_id: HashMap::new(),
        }
    }
    
    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn insert(&mut self, transaction: BankTransaction) -> bool {
        let mut hasher = DefaultHasher::new();
        transaction.hash(&mut hasher);
        
        if self.all.insert(hasher.finish()) {
            self.by_id.insert(transaction.id, transaction);
            true
        } else {
            false
        }
    }
    
    pub fn remove(&mut self, id: Uuid) -> bool {
        if let Some(transaction) = self.by_id.remove(&id) {
            let mut hasher = DefaultHasher::new();
            transaction.hash(&mut hasher);
            self.all.remove(&hasher.finish())
        } else {
            false
        }
    }
    
    pub fn check_hash(&self,hash: &u64) -> bool {
        self.all.contains(hash)
    }
    
    pub fn can_insert(&self, hash: &u64) -> bool {
        !self.check_hash(hash)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut BankTransaction> {
        self.by_id.get_mut(id)
    }
    
    pub fn contains(&self, id: &Uuid) -> bool {
        self.by_id.contains_key(id)
    }
}
// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_groups: HashMap<Uuid, BudgetGroup>,
    pub budget_items: HashMap<Uuid, BudgetItem>,
    pub bank_transactions: Store,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
}

impl Budget {
    pub fn get_item_mut(&mut self, item_id: &Uuid) -> Option<&mut BudgetItem> {
        self.budget_groups
            .iter_mut()
            .flat_map(move |(_, group)| group.items.iter_mut())
            .find(|item| item.id == *item_id)
    }

    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self.budget_groups
            .iter()
            .flat_map(move |(_, group)| group.items.iter())
            .find(|item| item.id == *item_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetGroup {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<BudgetItem>,
}

impl BudgetGroup {
    pub fn new(id: Uuid, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            items: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: Money,
    pub actual_spent: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetItemType {
    Income,
    Expense,
    Savings,
}

impl Display for BudgetItemType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BudgetItemType::Income => "Inkomst",
                BudgetItemType::Expense => "Utgift",
                BudgetItemType::Savings => "Sparande",
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BankAccount {
    pub id: Uuid,
    pub account_number: String,
    pub description: String,
    pub currency: String,
    pub balance: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct BankTransaction {
    pub id: Uuid,
    pub account_number: String,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub budget_item_id: Option<Uuid>,
    pub balance: Money,
}

impl PartialEq for BankTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount
            && self.description == other.description
            && self.date == other.date
    }

    // fn ne(&self, other: &Self) -> bool {
    //     self.amount != other.amount || self.description != other.description || self.date != other.date
    // }
}

impl Hash for BankTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.amount.hash(state);
        self.balance.hash(state);
        self.account_number.hash(state);
        self.description.hash(state);
        self.date.hash(state);
    }
}

impl BankTransaction {
    pub fn get_hash(&self) -> u64 {
        get_transaction_hash(&self.amount, &self.balance, &self.account_number, &self.description, &self.date)
    }
}

pub fn get_transaction_hash(amount: &Money, balance: &Money, account_number: &str, description: &str, date: &DateTime<Utc>) -> u64 {
    let mut hasher = DefaultHasher::new();
    amount.hash(&mut hasher);
    balance.hash(&mut hasher);
    account_number.hash(&mut hasher);
    description.hash(&mut hasher);
    date.hash(&mut hasher);
    hasher.finish()
}

impl Display for BankTransaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}, {}, {}", self.description, self.amount, self.date)
    }
}

impl BudgetItem {
    pub fn new(
        name: &str,
        item_type: BudgetItemType,
        budgeted_amount: Money,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            item_type,
            budgeted_amount,
            actual_spent: Money::new_dollars(0, budgeted_amount.currency()),
            notes,
            tags: tags.unwrap_or_default(),
        }
    }
}

impl BankTransaction {
    pub fn new(
        id: Uuid,
        account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_number: account_number.to_string(),
            amount,
            balance,
            description: description.to_string(),
            date,
            budget_item_id: None
        }
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn _new(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            user_id: Uuid::new_v4(),
            default_budget: false,
            budget_groups: HashMap::new(),
            budget_items: HashMap::new(),
            bank_transactions: Store::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_event: 0,
            version: 0,
        }
    }

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>) {
        if self.last_event < timestamp {
            self.last_event = timestamp;
            self.updated_at = updated_at;
            if self.version == 0 {
                self.created_at = updated_at;
            }
            self.version += 1;
        } else {
            panic!("Event timestamp is older than last event timestamp");
        }
    }

    fn _version(&self) -> u64 {
        self.version
    }
}
