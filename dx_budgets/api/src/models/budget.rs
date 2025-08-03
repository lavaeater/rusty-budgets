use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::Display;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use joydb::Model;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum MonthBeginsOn {
    PreviousMonth(u32),
    CurrentMonth(u32),
}

impl Default for MonthBeginsOn {
    fn default() -> Self {
        MonthBeginsOn::PreviousMonth(25)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub month_begins_on: MonthBeginsOn,
    pub budget_items: HashMap<BudgetAccountType, BudgetAccount>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub user_id: Uuid,
}

impl Budget {
    pub fn new(name: &str, default_budget: bool, user_id: Uuid) -> Budget {
        Budget {
            id: Uuid::new_v4(),
            name: name.to_string(),
            default_budget,
            month_begins_on: MonthBeginsOn::PreviousMonth(25),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            user_id,
            ..Default::default()
        }
    }
    
    pub fn store_budget_account(&mut self, budget_item: &BudgetAccount) {
        match self.budget_items.entry(budget_item.budget_account_type.clone()) {
            Vacant(e) => {
                e.insert(budget_item.clone());
            }
            Occupied(mut e) => {
                e.insert(budget_item.clone());
            }
        }
        self.touch();
    }
    
    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BankTransaction {
    pub id: Uuid,
    pub text: String,
    pub amount: f32,
    pub budget_item: Uuid,
    pub bank_date: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl BankTransaction {
    pub fn new_from_user(
        text: &str,
        amount: f32,
        budget_item: Uuid,
        bank_date: chrono::NaiveDate,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            amount,
            budget_item,
            bank_date,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum BudgetAccountType {
    Income(String),
    Expense(String),
    Savings(String),
}

impl Default for BudgetAccountType {
    fn default() -> Self {
        BudgetAccountType::Expense("Övrigt".to_string())
    }
}

impl Display for BudgetAccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetAccountType::Income(category) => write!(f, "Income: {}", category),
            BudgetAccountType::Expense(category) => write!(f, "Expense: {}", category),
            BudgetAccountType::Savings(category) => write!(f, "Savings: {}", category),
        }
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemPeriodicity {
    Once,
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

impl Display for BudgetItemPeriodicity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetItemPeriodicity::Once => write!(f, "Once"),
            BudgetItemPeriodicity::Monthly => write!(f, "Monthly"),
            BudgetItemPeriodicity::Quarterly => write!(f, "Quarterly"),
            BudgetItemPeriodicity::Yearly => write!(f, "Yearly"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetAccount {
    pub id: Uuid,
    pub budget_account_type: BudgetAccountType,
    pub incoming_transactions: HashMap<Uuid, BudgetTransaction>,
    pub outgoing_transactions: HashMap<Uuid, BudgetTransaction>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl BudgetAccount {
    pub fn new(
        budget_id: Uuid,
        budget_item_type: BudgetAccountType,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            budget_id,
            budget_account_type: budget_item_type,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
            ..Default::default()
        }
    }

    pub fn put_money_towards(&mut self, target: &mut BudgetAccount, amount: f32, text: &str) {
        let transaction = BudgetTransaction::new(
            text,
            BudgetTransactionType::default(),
            amount,
            Some(self.id),
            target.id,
            self.created_by,
        );
        self.store_outgoing_transaction(&transaction);
        target.store_incoming_transaction(&transaction);
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

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetTransactionType {
    #[default]
    StartValue,
    Adjustment,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetTransaction {
    pub id: Uuid,
    pub text: String,
    pub transaction_type: BudgetTransactionType,
    pub amount: f32,
    pub from_budget_item: Option<Uuid>,
    pub to_budget_item: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl BudgetTransaction {
    pub fn new(
        text: &str,
        transaction_type: BudgetTransactionType,
        amount: f32,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            transaction_type,
            amount,
            to_budget_item,
            from_budget_item,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}
