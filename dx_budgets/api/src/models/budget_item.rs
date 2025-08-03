use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Display;
use joydb::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::bank_transaction::BankTransaction;
use crate::models::budget_transaction::BudgetTransaction;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemType {
    Income,
    #[default]
    Expense,
    Savings,
}

impl Display for BudgetItemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetItemType::Income => write!(f, "Income"),
            BudgetItemType::Expense => write!(f, "Expense"),
            BudgetItemType::Savings => write!(f, "Savings"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemPeriodicity {
    Once,
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budget_item_type: BudgetItemType,
    pub periodicity: BudgetItemPeriodicity,
    pub starts_date: chrono::NaiveDate,
    pub incoming_transactions: HashMap<Uuid, BudgetTransaction>,
    pub outgoing_transactions: HashMap<Uuid, BudgetTransaction>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl BudgetItem {
    pub fn new_from_user(
        budget_id: Uuid,
        name: &str,
        budget_item_type: BudgetItemType,
        periodicity: BudgetItemPeriodicity,
        starts_date: chrono::NaiveDate,
        created_by: &Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            name: name.to_string(),
            budget_item_type,
            periodicity,
            starts_date,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by: *created_by,
            ..Default::default()
        }
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
