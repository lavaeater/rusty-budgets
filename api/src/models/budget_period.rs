use crate::models::budget_period_id::PeriodId;
use crate::models::{ActualItem, BankTransaction, BudgetItem, MatchRule};
use core::fmt::Display;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;
#[derive(Debug, Clone)]
pub struct BudgetPeriod {
    pub id: PeriodId,
    pub actual_items: Vec<ActualItem>,
    pub transactions: Vec<BankTransaction>,
}

impl BudgetPeriod {
    pub fn mutate_actual(&mut self, actual_id: Uuid, mut mutator: impl FnMut(&mut ActualItem)) {
        if let Some(actual) = self.get_actual_mut(actual_id) {
            mutator(actual);
        }
    }
    pub fn get_actual(&self, id: Uuid) -> Option<&ActualItem> {
        self.actual_items.iter().find(|i| i.id == id)
    }

    pub fn get_actual_mut(&mut self, id: Uuid) -> Option<&mut ActualItem> {
        self.actual_items.iter_mut().find(|i| i.id == id)
    }
    pub fn add_actual(&mut self, actual_item: ActualItem) {
        self.actual_items.push(actual_item);
    }
    pub fn contains_actual_for_item(&self, item_id: Uuid) -> bool {
        self.actual_items
            .iter()
            .any(|i| i.budget_item_id == item_id)
    }

    pub fn contains_actual(&self, actual_id: Uuid) -> bool {
        self.actual_items.iter().any(|i| i.id == actual_id)
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
            actual_items: Vec::new(),
            transactions: Vec::new(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>, items: &[BudgetItem]) -> Vec<(Uuid, Option<Uuid>, Option<Uuid>)> {
        let mut matched_transactions = Vec::new();
        let actuals = self.actual_items.iter().collect::<Vec<_>>();
        for transaction in self.transactions.iter() {
            for rule in rules {
                if rule.matches_transaction(&transaction) {
                    if let Some(actual_id) = self.get_actual_id_for_rule(rule, &actuals) {
                        matched_transactions.push((transaction.id, Some(actual_id), None));
                        break;
                    } else if let Some(item_id) = self.get_item_id_for_rule(rule, items) {
                        matched_transactions.push((transaction.id, None, Some(item_id)));
                        break;
                    }
                }
            }
        }
        matched_transactions
    }

    pub fn get_actual_id_for_rule(&self, rule: &MatchRule, actuals: &Vec<&ActualItem>) -> Option<Uuid> {
        actuals.iter().find(|i| rule.matches_actual(i)).map(|i| i.id)
    }

    pub fn get_item_id_for_rule(&self, rule: &MatchRule, items: &[BudgetItem]) -> Option<Uuid> {
        items.iter().find(|i| rule.matches_item(i)).map(|i| i.id)
    }
    
    pub fn all_actual_items(&self) -> &Vec<ActualItem> {
        &self.actual_items
    }
}
