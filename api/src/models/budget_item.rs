use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use joydb::Model;
use crate::models::bank_transaction::BankTransaction;
use crate::models::budget_transaction::BudgetTransaction;
use crate::models::budget_category::BudgetCategory;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budget_category: BudgetCategory,
    pub incoming_transactions: HashMap<Uuid, BudgetTransaction>,
    pub outgoing_transactions: HashMap<Uuid, BudgetTransaction>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl Hash for BudgetItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl BudgetItem {
    pub fn new(
        budget_id: Uuid,
        name: &str,
        budget_category: &BudgetCategory,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            name: name.to_string(),
            budget_category: budget_category.clone(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
            ..Default::default()
        }
    }
    
    pub fn incoming_amount(&self) -> f32 {
        self.incoming_transactions.values().map(|v| v.amount)
            .sum::<f32>()
    }
    
    pub fn outgoing_amount(&self) -> f32 {
        self.outgoing_transactions.values().map(|v| v.amount)
            .sum::<f32>()
    }
    
    pub fn budgeted_amount(&self) -> f32 {
        self.incoming_amount() - self.outgoing_amount()
    }
    
    pub fn total_bank_amount(&self) -> f32 {
        self.bank_transactions.values().map(|v| v.amount)
            .sum::<f32>()
    }

    pub fn store_incoming_transaction(&mut self, budget_transaction: &BudgetTransaction) {
        match self.incoming_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn store_outgoing_transaction(&mut self, budget_transaction: &BudgetTransaction) {
        match self.outgoing_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn store_bank_transaction(&mut self, budget_transaction: &BankTransaction) {
        match self.bank_transactions.entry(budget_transaction.id) {
            Vacant(e) => {
                e.insert(budget_transaction.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_transaction.clone());
            }
        }
        self.touch();
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}