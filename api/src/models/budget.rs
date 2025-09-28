use crate::cqrs::framework::DomainEvent;
use crate::events::item_funds_adjusted::ItemFundsAdjusted;
use crate::cqrs::framework::Aggregate;
use crate::models::money::{Currency, Money};
use crate::pub_events_enum;
use chrono::{DateTime, Utc};
use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use crate::events::budget_created::BudgetCreated;
use crate::events::item_added::ItemAdded;
use crate::events::item_funds_reallocated::ItemFundsReallocated;
use crate::events::transaction_added::TransactionAdded;
use crate::events::transaction_connected::TransactionConnected;
use crate::models::bank_transaction::BankTransactionStore;
use crate::models::budget_item::{BudgetItem, BudgetItemStore};
use crate::models::budgeting_type::BudgetingType;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{BudgetingTypeOverview, Rule, ValueKind};
use crate::models::Rule::{Difference, Sum};

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
    pub actual_by_type: HashMap<BudgetingType, Money>,
    pub budgeting_overview: HashMap<BudgetingType, BudgetingTypeOverview>,
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
            budgeting_overview: HashMap::from([
                (BudgetingType::Expense, BudgetingTypeOverview::default()),
                (BudgetingType::Savings, BudgetingTypeOverview::default()),
                (BudgetingType::Income, BudgetingTypeOverview::default()),
            ]),
            budgeted_by_type: HashMap::from([
                (BudgetingType::Expense, Money::default()),
                (BudgetingType::Savings, Money::default()),
                (BudgetingType::Income, Money::default()),
            ]),
            actual_by_type:HashMap::from([
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

    pub fn get_type_for_item(&self, item_id: &Uuid) -> Option<&BudgetingType> {
        self.budget_items.type_for(item_id)
    }

    pub fn get_item_mut(&mut self, item_id: &Uuid) -> Option<&mut BudgetItem> {
        self.budget_items.get_mut(item_id)
    }
    
    pub fn items_by_type(&self) -> Vec<(usize, BudgetingType, Vec<BudgetItem>)> {
        self.budget_items.items_by_type()
    }
    
    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_items.by_type(budgeting_type).unwrap_or_default().iter().map(|item| item.budgeted_amount).sum()
    }
    
    pub fn spent_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.budget_items.by_type(budgeting_type).unwrap_or_default().iter().map(|item| item.actual_amount).sum()
    }
    
    pub fn recalculate(&mut self) {
        let income_sum = Sum(vec![Income]);
        let budgeted_income = income_sum.evaluate(&self.budget_items.hash_by_type(), ValueKind::Budgeted);
        let spent_income = income_sum.evaluate(&self.budget_items.hash_by_type(), ValueKind::Spent);        
        let remaining_rule = Difference(Income, vec![Expense, Savings]);
        let remaining_income = remaining_rule.evaluate(&self.budget_items.hash_by_type(), ValueKind::Budgeted);
        
        let income_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_income,
            actual_amount: spent_income,
            remaining_budget: remaining_income,
        };
        
        self.budgeting_overview.insert(Income, income_overview);
        
        let expense_sum = Sum(vec![Expense]);
        let budgeted_expenses = expense_sum.evaluate(&self.budget_items.hash_by_type(), ValueKind::Budgeted);
        let spent_expenses = expense_sum.evaluate(&self.budget_items.hash_by_type(), ValueKind::Spent);

        let remaining_expenses = remaining_rule.evaluate(&self.budget_items.hash_by_type(), ValueKind::Budgeted);

        //1. How much do we have left to budget (i.e. Income - Expenses - Savings)
        //2. How much do have )
        // self.budgeted_by_type
        //     .entry(event.item_type)
        //     .and_modify(|v| {
        //         *v += event.budgeted_amount;
        //     })
        //     .or_insert(event.budgeted_amount);
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
