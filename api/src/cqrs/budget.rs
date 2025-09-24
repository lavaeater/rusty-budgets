use crate::cqrs::framework::DomainEvent;
use crate::cqrs::domain_events::{BudgetCreated, GroupAdded, ItemAdded, ItemFundsAdjusted, ItemFundsReallocated, TransactionAdded, TransactionConnected};
use crate::cqrs::framework::Aggregate;
use crate::cqrs::money::{Currency, Money};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use uuid::Uuid;

/// Generic module for Arc serialization/deserialization helpers
pub mod arc_helpers {
    use super::*;

    // HashMap<K, Arc<V>>
    pub fn deserialize_hashmap_arc<'de, D, K, V>(deserializer: D) -> Result<HashMap<K, Arc<V>>, D::Error>
    where
        D: Deserializer<'de>,
        K: Deserialize<'de> + std::hash::Hash + Eq,
        V: Deserialize<'de>,
    {
        let map: HashMap<K, V> = HashMap::deserialize(deserializer)?;
        Ok(map.into_iter().map(|(k, v)| (k, Arc::new(v))).collect())
    }

    pub fn serialize_hashmap_arc<S, K, V>(map: &HashMap<K, Arc<V>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        K: Serialize + std::hash::Hash + Eq,
        V: Serialize,
    {
        let map_ref: HashMap<&K, &V> = map.iter().map(|(k, v)| (k, v.as_ref())).collect();
        map_ref.serialize(serializer)
    }

    // Vec<Arc<V>>
    pub fn deserialize_vec_arc<'de, D, V>(deserializer: D) -> Result<Vec<Arc<V>>, D::Error>
    where
        D: Deserializer<'de>,
        V: Deserialize<'de>,
    {
        let vec: Vec<V> = Vec::deserialize(deserializer)?;
        Ok(vec.into_iter().map(Arc::new).collect())
    }

    pub fn serialize_vec_arc<S, V>(vec: &Vec<Arc<V>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        V: Serialize,
    {
        let vec_ref: Vec<&V> = vec.iter().map(|v| v.as_ref()).collect();
        vec_ref.serialize(serializer)
    }
}

/// The store
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BudgetItemStore {
    #[serde(
        deserialize_with = "arc_helpers::deserialize_hashmap_arc",
        serialize_with = "arc_helpers::serialize_hashmap_arc"
    )]
    items: HashMap<Uuid, Arc<BudgetItem>>,

    #[serde(
        deserialize_with = "arc_helpers::deserialize_vec_arc",
        serialize_with = "arc_helpers::serialize_vec_arc"
    )]
    by_type: HashMap<BudgetingType, Vec<Arc<BudgetItem>>>,
}

pub_events_enum! {
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum BudgetEvent {
        BudgetCreated,
        ItemAdded,
        TransactionAdded,
        TransactionConnected,
        ItemFundsReallocated,
        ItemFundsAdjusted,
    }
}

impl BudgetItemStore {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn insert(&mut self, item: &BudgetItem) -> bool {
        self.by_type.entry(item.budgeting_type).and_modify(|e| {
            e.push(item);
        })
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


#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionStore {
    all: HashSet<u64>,       // uniqueness check
    by_id: HashMap<Uuid, BankTransaction> // fast lookup
}


impl BankTransactionStore {
    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
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
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_items_by_type: HashMap<BudgetingType, BudgetItem>,
    pub bank_transactions: BankTransactionStore,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
    pub currency: Currency,
    pub budgeted_by_type: HashMap<BudgetingType, Money>, 
    pub spent_by_type: HashMap<BudgetingType, Money>, 
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: "".to_string(),
            user_id: Default::default(),
            budget_items: Default::default(),
            bank_transactions: Default::default(),
            created_at: Default::default(),
            updated_at: Default::default(),
            default_budget: false,
            last_event: 0,
            version: 0,
            currency: Default::default(),
            budgeted_by_type: HashMap::from([
                (BudgetingType::Expense, Money::default()),
                (BudgetingType::Savings, Money::default()),
                (BudgetingType::Income, Money::default()),
            ]),
            spent_by_type:HashMap::from([
                (BudgetingType::Expense, Money::default()),
                (BudgetingType::Savings, Money::default()),
                (BudgetingType::Income, Money::default()),
            ]),
            
        }
    }
}

impl Budget {
    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        if let  Some(group_id) = self.budget_items_and_groups.get(item_id) {
            // Update group
            if let Some(group) = self.budget_groups.get(group_id) {
                return group.items.iter().find(|item| item.id == *item_id)
            }
        }
        None
    }

    pub fn get_group_mut(&mut self, group_id: &Uuid) -> Option<&mut BudgetGroup> {
        self.budget_groups.get_mut(group_id)
    }
    
    pub fn get_group_mut_for_item_id(&mut self, item_id: &Uuid) -> Option<&mut BudgetGroup> {
        if let Some(group_id) = self.budget_items_and_groups.get(item_id) {
            return self.budget_groups.get_mut(group_id)
        }
        None
    }

    pub fn get_group_for_item_id(&self, item_id: &Uuid) -> Option<&BudgetGroup> {
        if let Some(group_id) = self.budget_items_and_groups.get(item_id) {
            return self.budget_groups.get(group_id)
        }
        None
    }

    pub fn get_item_mut(&mut self, item_id: &Uuid) -> Option<&mut BudgetItem> {
        if let  Some(group_id) = self.budget_items_and_groups.get(item_id) {
            // Update group
            if let Some(group) = self.budget_groups.get_mut(group_id) {
                return group.items.iter_mut().find(|item| item.id == *item_id)
            }
        }
        None
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetGroup {
    pub id: Uuid,
    pub name: String,
    pub group_type: BudgetingType,
    pub items: Vec<BudgetItem>,
    pub budgeted_amount: Money,
    pub actual_spent: Money,
}

impl BudgetGroup {
    pub fn new(id: Uuid, name: &str, group_type: BudgetingType, currency: Currency) -> Self {
        Self {
            id,
            name: name.to_string(),
            group_type,
            budgeted_amount: Money::new_cents(0, currency),
            actual_spent: Money::new_cents(0, currency),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_spent: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Default,Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BudgetingType {
    #[default]
    Income,
    Expense,
    Savings,
}

impl Display for BudgetingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BudgetingType::Income => "Inkomst",
                BudgetingType::Expense => "Utgift",
                BudgetingType::Savings => "Sparande",
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
        id: Uuid,
        name: &str,
        item_type: BudgetingType,
        budgeted_amount: Money,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            id,
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
            ..Self::default()
        }
    }
    
    fn _default() -> Self {
        Self::default()
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
