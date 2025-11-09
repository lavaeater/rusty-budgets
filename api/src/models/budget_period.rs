use core::fmt::Display;
use crate::models::bank_transaction_store::BankTransactionStore;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{ActualItem, BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule, Money};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use dioxus::logger::tracing;
use serde::ser::SerializeStruct;
use uuid::Uuid;
use crate::models::budget_item_store::BudgetItemStore;
use crate::models::budget_period_id::PeriodId;

#[derive(Debug, Clone)]
pub struct BudgetPeriod {
    pub id: PeriodId,
    pub actual_items: HashMap<Uuid, ActualItem>,
    pub transactions: BankTransactionStore,
}

impl Serialize for BudgetPeriod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BudgetPeriod", 3)?;
        state.serialize_field("id", &self.id)?;
        let actual_items: HashMap<String, ActualItem> = self
            .actual_items
            .iter()
            .map(|(k, v)| { 
                (k.to_string(), v.clone()) }
            )
            .collect();
        state.serialize_field("actual_items", &actual_items)?;
        state.serialize_field("transactions", &self.transactions)?;
        state.end()
    }
}

impl BudgetPeriod {
    pub fn mutate_actual(&mut self, actual_id: Uuid, mut mutator: impl FnMut(&mut ActualItem)) {
        if let Some(actual) = self.get_actual_mut(actual_id) {
            mutator(actual);
        }
    }
    pub fn get_actual(&self, id: Uuid) -> Option<&ActualItem> {
        self.actual_items.get(&id)
    }

    pub fn get_actual_mut(&mut self, id: Uuid) -> Option<&mut ActualItem> {
        self.actual_items.get_mut(&id)
    }
    
    pub fn add_actual(&mut self, actual_item: ActualItem) {
        self.actual_items.insert(actual_item.id, actual_item);
    }
    pub fn contains_actual_for_item(&self, item_id: Uuid) -> bool {
       self.actual_items.values().any(|i| i.budget_item_id.borrow().id == item_id)
    }
    fn clear_hashmaps_and_transactions(&mut self) {
        self.transactions.clear();
    }
    pub fn clone_to(&self, id: PeriodId) -> Self {
        let mut period = self.clone();
        period.id = id;
        period.clear_hashmaps_and_transactions();
        period
    }
    pub fn new_for(id: PeriodId) -> Self {
        let mut period = Self {
            id,
            actual_items: HashMap::new(),
            transactions: BankTransactionStore::default(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>) -> Vec<(Uuid, Uuid)> {
        let mut matched_transactions = Vec::new();
        let items = self.actual_items.values().collect();
        for transaction in self.transactions.list_transactions_for_connection() {
            for rule in rules {
                if rule.matches_transaction(&transaction) {
                    if let Some(item_id) = self.get_item_for_rule(rule, &items) {
                        matched_transactions.push((transaction.id, item_id));
                        break;
                    }
                }
            }
        }
        matched_transactions
    }

    pub fn get_item_for_rule(&self, rule: &MatchRule, items: &Vec<&ActualItem>) -> Option<Uuid> {
        items.iter().find(|i| rule.matches_item(i)).map(|i| i.id)
    }
}

