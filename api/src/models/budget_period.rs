use core::fmt::Display;
use crate::models::bank_transaction_store::BankTransactionStore;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule, Money};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use crate::models::budget_item_store::BudgetItemStore;
use crate::models::budget_period_id::BudgetPeriodId;

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub id: BudgetPeriodId,
    pub budget_items: BudgetItemStore,
    pub transactions: BankTransactionStore,
    pub budgeted_by_type: HashMap<BudgetingType, Money>,
    pub actual_by_type: HashMap<BudgetingType, Money>,
    pub budgeting_overview: HashMap<BudgetingType, BudgetingTypeOverview>,
}

impl BudgetPeriod {
    fn clear_hashmaps_and_transactions(&mut self) {
        self.transactions.clear();
        self.budgeting_overview = HashMap::from([
            (Expense, BudgetingTypeOverview::default()),
            (Savings, BudgetingTypeOverview::default()),
            (Income, BudgetingTypeOverview::default()),
        ]);
        self.budgeted_by_type = HashMap::from([
            (Expense, Money::default()),
            (Savings, Money::default()),
            (Income, Money::default()),
        ]);
        self.actual_by_type = HashMap::from([
            (Expense, Money::default()),
            (Savings, Money::default()),
            (Income, Money::default()),
        ]);
    }
    pub(crate) fn clone_to(&self, id: BudgetPeriodId) -> Self {
        let mut period = self.clone();
        period.id = id;
        period.clear_hashmaps_and_transactions();
        period
    }
    pub(crate) fn new_for(id: BudgetPeriodId) -> Self {
        let mut period = Self {
            id: id,
            budget_items: BudgetItemStore::default(),
            transactions: BankTransactionStore::default(),
            budgeted_by_type: Default::default(),
            actual_by_type: Default::default(),
            budgeting_overview: Default::default(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>, items: &Vec<BudgetItem>) -> Vec<(Uuid, Uuid)> {
        let mut matched_transactions = Vec::new();
        for transaction in self.transactions.list_transactions_for_connection() {
            for rule in rules {
                if rule.matches_transaction(&transaction) {
                    if let Some(item_id) = self.get_item_for_rule(rule, items) {
                        matched_transactions.push((transaction.id, item_id));
                        break;
                    }
                }
            }
        }
        matched_transactions
    }

    pub fn get_item_for_rule(&self, rule: &MatchRule, items: &Vec<BudgetItem>) -> Option<Uuid> {
        items.iter().find(|i| rule.matches_item(i)).map(|i| i.id)
    }
    
    pub fn update_actual_amount(&mut self, budgeting_type: &BudgetingType, amount: &Money) {
        self.actual_by_type
            .entry(*budgeting_type)
            .and_modify(|v| *v += *amount);
    }
    
    pub fn update_budgeted_amount(&mut self, budgeting_type: &BudgetingType, amount: &Money) {
        self.budgeted_by_type
            .entry(*budgeting_type)
            .and_modify(|v| *v += *amount);
    }
    
    pub fn add_actual_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_items.add_actual_amount(item_id, amount);
    }
    
    pub fn add_budgeted_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.budget_items.add_budgeted_amount(item_id, amount);
    }
    
    pub fn insert_item(&mut self, item: &BudgetItem, budgeting_type: BudgetingType) -> bool {
        self.budget_items.insert(item, budgeting_type)
    }
}

