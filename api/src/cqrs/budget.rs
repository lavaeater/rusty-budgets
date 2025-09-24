use crate::cqrs::framework::DomainEvent;
use crate::cqrs::domain_events::{BudgetCreated, GroupAdded, ItemAdded, ItemFundsAdjusted, ItemFundsReallocated, TransactionAdded, TransactionConnected};
use crate::cqrs::framework::Aggregate;
use crate::cqrs::money::{Currency, Money};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use uuid::Uuid;

/// The store
#[derive(Default, Debug, Clone)]
pub struct BudgetItemStore {
    items: HashMap<Uuid, Arc<BudgetItem>>,
    by_type: HashMap<BudgetingType, Vec<Arc<BudgetItem>>>,
}

impl Serialize for BudgetItemStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        use serde::ser::SerializeStruct;
        
        let mut state = serializer.serialize_struct("BudgetItemStore", 2)?;
        
        // Serialize items by dereferencing the Arc to get the inner BudgetItem
        let items_map: HashMap<Uuid, &BudgetItem> = self.items
            .iter()
            .map(|(k, v)| (*k, v.as_ref()))
            .collect();
        state.serialize_field("items", &items_map)?;
        
        // Serialize by_type by dereferencing the Arc in each Vec
        let by_type_map: HashMap<BudgetingType, Vec<&BudgetItem>> = self.by_type
            .iter()
            .map(|(k, v)| (*k, v.iter().map(|arc| arc.as_ref()).collect()))
            .collect();
        state.serialize_field("by_type", &by_type_map)?;
        
        state.end()
    }
}

impl<'de> Deserialize<'de> for BudgetItemStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field { Items, ByType }

        struct BudgetItemStoreVisitor;

        impl<'de> Visitor<'de> for BudgetItemStoreVisitor {
            type Value = BudgetItemStore;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BudgetItemStore")
            }

            fn visit_map<V>(self, mut map: V) -> Result<BudgetItemStore, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut items: Option<HashMap<Uuid, BudgetItem>> = None;
                
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Items => {
                            if items.is_some() {
                                return Err(de::Error::duplicate_field("items"));
                            }
                            items = Some(map.next_value()?);
                        }
                        Field::ByType => {
                            // We'll ignore the by_type field during deserialization
                            // and reconstruct it from the items
                            let _: HashMap<BudgetingType, Vec<BudgetItem>> = map.next_value()?;
                        }
                    }
                }

                let items = items.ok_or_else(|| de::Error::missing_field("items"))?;
                
                // Reconstruct the store with Arc wrappers and dual indexing
                let mut store = BudgetItemStore::default();
                for (id, item) in items {
                    let item_arc = Arc::new(item);
                    store.items.insert(id, item_arc.clone());
                    store.by_type.entry(item_arc.item_type).or_insert_with(Vec::new).push(item_arc);
                }

                Ok(store)
            }
        }

        const FIELDS: &'static [&'static str] = &["items", "by_type"];
        deserializer.deserialize_struct("BudgetItemStore", FIELDS, BudgetItemStoreVisitor)
    }
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
        if !self.items.contains_key(&item.id) {
            let arc = Arc::new(item.clone());
            self.items.insert(item.id, arc.clone());
            self.by_type.entry(item.item_type).or_insert_with(Vec::new).push(arc);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: Uuid) -> bool {
        if self.items.contains_key(&id) {
            let arc = self.items.remove(&id).unwrap();
            self.by_type.entry(arc.item_type).and_modify(|v| {
                v.retain(|x| x.id != id);
            });
            true
        } else {
            false
        }
    }
    
    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut BudgetItem> {
        self.items.get_mut(id).and_then(|arc| Some(Arc::make_mut(arc)))
    }

    pub fn get(&self, id: &Uuid) -> Option<&BudgetItem> {
        self.items.get(id).and_then(|arc| Some(arc.as_ref()))
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        self.items.contains_key(id)
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
    pub budget_items: BudgetItemStore,
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
        self.budget_items.get(item_id)
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
