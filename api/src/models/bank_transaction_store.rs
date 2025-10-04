use crate::models::BankTransaction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionStore {
    hashes: HashSet<u64>,                  // uniqueness check
    by_id: HashMap<Uuid, BankTransaction>, // fast lookup
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

    pub fn check_hash(&self, hash: &u64) -> bool {
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
        let mut transactions = self.by_id.values().collect::<Vec<_>>();
        transactions.sort_by_key(|tx| tx.date);
        transactions
    }

    pub fn list_transactions_for_connection(&self) -> Vec<&BankTransaction> {
        let mut transactions = self
            .by_id
            .values()
            .filter(|tx| tx.budget_item_id.is_none())
            .collect::<Vec<_>>();
        transactions.sort_by_key(|tx| tx.date);
        transactions
    }
}
