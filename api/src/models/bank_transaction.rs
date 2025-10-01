use core::fmt;
use core::fmt::{Display, Formatter};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::models::money::Money;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionStore {
    hashes: HashSet<u64>,       // uniqueness check
    by_id: HashMap<Uuid, BankTransaction> // fast lookup
}

impl BankTransactionStore {
    
    pub fn clear(&mut self) {
        self.hashes.clear();
        self.by_id.clear();
    }
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    pub fn insert(&mut self, transaction: BankTransaction) -> bool {
        let mut hasher = DefaultHasher::new();
        transaction.hash(&mut hasher);

        if self.hashes.insert(hasher.finish()) {
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
            self.hashes.remove(&hasher.finish())
        } else {
            false
        }
    }

    pub fn check_hash(&self,hash: &u64) -> bool {
        self.hashes.contains(hash)
    }

    pub fn can_insert(&self, hash: &u64) -> bool {
        !self.check_hash(hash)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut BankTransaction> {
        self.by_id.get_mut(id)
    }
    
    pub fn get(&self, id: &Uuid) -> Option<&BankTransaction> {
        self.by_id.get(id)
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        self.by_id.contains_key(id)
    }
    
    pub fn list_transactions(&self) -> Vec<&BankTransaction> {
        self.by_id.values().collect()
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