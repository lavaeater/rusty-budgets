use core::fmt::Display;
use crate::models::bank_transaction_store::BankTransactionStore;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{ActualItem, BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule, Money};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use dioxus::logger::tracing;
use uuid::Uuid;
use crate::models::budget_item_store::BudgetItemStore;
use crate::models::budget_period_id::BudgetPeriodId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub id: BudgetPeriodId,
    pub items: HashMap<Uuid, ActualItem>,
    pub transactions: BankTransactionStore,
    pub budgeted_by_type: HashMap<BudgetingType, Money>,
    pub actual_by_type: HashMap<BudgetingType, Money>,
    // pub budgeting_overview: HashMap<BudgetingType, BudgetingTypeOverview>,
}

impl BudgetPeriod {
    pub fn add_actual(&mut self, actual_item: ActualItem) {
        self.items.insert(actual_item.id, actual_item);
    }
    pub fn contains_actual_for_item(&self, item_id: Uuid) -> bool {
       self.items.values().any(|i| i.budget_item.borrow().id == item_id)
    }
    fn clear_hashmaps_and_transactions(&mut self) {
        self.transactions.clear();
        // self.budgeting_overview = HashMap::from([
        //     (Expense, BudgetingTypeOverview::default()),
        //     (Savings, BudgetingTypeOverview::default()),
        //     (Income, BudgetingTypeOverview::default()),
        // ]);
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
    pub fn clone_to(&self, id: BudgetPeriodId) -> Self {
        let mut period = self.clone();
        period.id = id;
        period.clear_hashmaps_and_transactions();
        period
    }
    pub fn new_for(id: BudgetPeriodId) -> Self {
        let mut period = Self {
            id: id,
            items: HashMap::new(),
            transactions: BankTransactionStore::default(),
            budgeted_by_type: Default::default(),
            actual_by_type: Default::default(),
            // budgeting_overview: Default::default(),
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

    pub fn get_item_for_rule(&self, rule: &MatchRule, items: &Vec<ActualItem>) -> Option<Uuid> {
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
    
/* <<<<<<<<<<<<<<  ✨ Windsurf Command ⭐ >>>>>>>>>>>>>>>> */
    /// Adds the given amount to the actual amount of the item with the given ID.
    ///
    /// If the item with the given ID does not exist, this function does nothing.
    ///
    /// # Parameters
    ///
    /// * `item_id`: The ID of the item to update.
    /// * `amount`: The amount to add to the item's actual amount.
    ///
    /// # Return value
    ///
    /// This function does not return a value.
    ///
    /// # Examples
    ///
    /// 
/* <<<<<<<<<<  fa5f07c7-b578-42e5-8eb7-f3c613f98a55  >>>>>>>>>>> */
    pub fn add_actual_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        let bork = self.items.get_mut(item_id);
        match bork {
            
        }
        if let Some(item) = self.items.get(item_id) {
            self.update_actual_amount(&item.budget_item.borrow().budgeting_type, amount);
        }
    }
    
    pub fn add_budgeted_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        if let Some(item) = self.items.get_mut(item_id) {
            item.budgeted_amount += *amount;
            self.update_budgeted_amount(&item.budget_item.borrow().budgeting_type, amount);
        }
    }
}

