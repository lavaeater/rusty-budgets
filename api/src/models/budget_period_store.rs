// use crate::models::budget_period::BudgetPeriod;
// use crate::models::budget_period_id::PeriodId;
// use crate::models::BudgetingType::{Expense, Income, Savings};
// use crate::view_models::Rule::{Difference, SelfDiff, Sum};
// use crate::models::{
//     ActualItem, BankTransaction, BudgetItem, BudgetingType, MatchRule,
//     Money, MonthBeginsOn,
// };
// use anyhow::__private::NotBothDebug;
// use chrono::{DateTime, Utc};
// use iter_tools::Itertools;
// use serde::{Deserialize, Serialize};
// use std::collections::{HashMap, HashSet};
// use std::sync::{Arc, Mutex};
// use dioxus::logger::tracing;
// use uuid::Uuid;
// use crate::view_models::{BudgetingTypeOverview, Rule, ValueKind};
// 
// 
//     pub fn with_period(&self, id: PeriodId) -> Option<&BudgetPeriod> {
//         self.budget_periods.get(&id)
//     }
// 
//     pub fn with_period_mut(&mut self, id: PeriodId) -> Option<&mut BudgetPeriod> {
//         self.budget_periods.get_mut(&id)
//     }
// 
//     pub fn list_ignored_transactions(&self, period_id: PeriodId) -> Vec<BankTransaction> {
//         self.with_period(period_id)
//             .map(|p| p.transactions.list_ignored_transactions())
//             .unwrap_or_default()
//     }
// 
//     pub(crate) fn month_begins_on(&self) -> MonthBeginsOn {
//         self.month_begins_on
//     }
// 
//     pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>, items: &Vec<BudgetItem>) -> Vec<(Uuid, Option<Uuid>, Option<Uuid>)> {
//         tracing::info!("What is up with the rules, bro? {:#?}", rules);
//         self.budget_periods
//             .iter()
//             .flat_map(|(_, period)| period.evaluate_rules(rules, items))
//             .collect::<Vec<_>>()
//     }
// 
//     pub fn move_transaction_to_ignored(&mut self, tx_id: Uuid, period_id: PeriodId) -> bool {
//         self.with_period_mut(period_id)
//             .map(|p| p.transactions.ignore_transaction(tx_id))
//             .unwrap_or_default()
//     }
// 
//     pub fn get_period_before(&self, id: PeriodId) -> Option<&BudgetPeriod> {
//         if self.budget_periods.is_empty() {
//             return None;
//         }
//         self.budget_periods
//             .keys()
//             .filter(|key| key.year < id.year || (key.year == id.year && key.month < id.month))
//             .max()
//             .map(|key| self.budget_periods.get(key).unwrap())
//     }
// 
//     pub fn create_period_before(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
//         let period = period_id.month_before();
//         self.get_or_create_periodget_or_create_period(period)
//     }
// 
//     pub fn create_period_after(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
//         let period = period_id.month_after();
//         self.get_or_create_period(period)
//     }
// 
//     pub fn get_period_for_date_mut(&mut self, date: &DateTime<Utc>) -> &mut BudgetPeriod {
//         self.get_or_create_period(PeriodId::from_date(*date, self.month_begins_on))
//     }
// 
//     pub fn get_period_for_date(&mut self, date: &DateTime<Utc>) -> &BudgetPeriod {
//         self.get_or_create_period(PeriodId::from_date(*date, self.month_begins_on))
//     }
// 
//     fn get_or_create_period(&mut self, id: PeriodId) -> &mut BudgetPeriod {
//         if self.budget_periods.contains_key(&id) {
//             return self.budget_periods.get_mut(&id).unwrap();
//         }
//         let previous_period = self.get_period_before(id);
//         let period = if let Some(previous_period) = previous_period {
//             previous_period.clone_to(id)
//         } else {
//             BudgetPeriod::new_for(id)
//         };
//         self.budget_periods.insert(id, period);
//         self.budget_periods.get_mut(&id).unwrap()
//     }
// 
//     pub fn items_by_type(
//         &self,
//         period_id: PeriodId,
//     ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<ActualItem>)> {
//         let b = self
//             .with_period(period_id)
//             .map(|p| {
//                 p.actual_items
//                     .values()
//                     .group_by(|item| item.budgeting_type())
//                     .into_iter()
//                     .enumerate()
//                     .map(|(index, (group, items))| {
//                         let overview = match group {
//                             Income => self.get_income_overview(period_id),
//                             Expense => self.get_expense_overview(period_id),
//                             Savings => self.get_savings_overview(period_id),
//                         };
//                         (index, group, overview, items.cloned().collect::<Vec<_>>())
//                     })
//                     .collect::<Vec<_>>()
//             })
//             .unwrap_or_default();
//         b
//     }
// 
//     pub fn list_items_of_type(
//         &self,
//         budgeting_type: BudgetingType,
//         period_id: PeriodId,
//     ) -> Vec<&ActualItem> {
//         self.with_period(period_id)
//             .map(|p| {
//                 p.actual_items
//                     .values()
//                     .filter(|item| item.budgeting_type() == budgeting_type)
//                     .collect::<Vec<_>>()
//             })
//             .unwrap_or_default()
//     }
// 
//     pub fn budgeted_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
//         self.list_items_of_type(budgeting_type, period_id)
//             .iter()
//             .map(|item| item.budgeted_amount)
//             .sum()
//     }
// 
//     pub fn spent_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
//         self.list_items_of_type(budgeting_type, period_id)
//             .iter()
//             .map(|item| item.actual_amount)
//             .sum()
//     }
// 
//     pub fn get_income_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
//         let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Income).unwrap();
//         tracing::info!("Rules: {:?}", rules);
//         
//         let items = &self.list_items_of_type(Income, period_id);
//         let budgeted_income = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
//         let spent_income = rules.actual_rule.evaluate(items, Some(ValueKind::Spent));
//         let remaining_income = rules.remaining_rule.evaluate(items, Some(ValueKind::Budgeted));
// 
//         BudgetingTypeOverview {
//             budgeting_type: Income,
//             budgeted_amount: budgeted_income,
//             actual_amount: spent_income,
//             remaining_budget: remaining_income,
//             is_ok: remaining_income == Money::zero(remaining_income.currency()),
//         }
//     }
// 
//     pub fn get_expense_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
//         let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Expense).unwrap();
//         let items = &self.list_items_of_type(Expense, period_id);
//         let budgeted_expenses = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
//         let spent_expenses = rules.actual_rule.evaluate(items, Some(ValueKind::Spent));
//         let self_diff = rules.remaining_rule.evaluate(items, None);
// 
//         BudgetingTypeOverview {
//             budgeting_type: Expense,
//             budgeted_amount: budgeted_expenses,
//             actual_amount: spent_expenses,
//             remaining_budget: self_diff,
//             is_ok: self_diff < Money::zero(self_diff.currency()),
//         }
//     }
// 
//     pub fn get_savings_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
//         let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Savings).unwrap();
//         let items = &self.list_items_of_type(Savings, period_id);
//         let budgeted_savings = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
//         let spent_savings = rules.actual_rule.evaluate(items, Some(ValueKind::Spent));
//         let self_diff = rules.remaining_rule.evaluate(items, None);
// 
//         BudgetingTypeOverview {
//             budgeting_type: Savings,
//             budgeted_amount: budgeted_savings,
//             actual_amount: spent_savings,
//             remaining_budget: self_diff,
//             is_ok: self_diff < Money::zero(self_diff.currency()),
//         }
//     }
// 
//     pub fn insert_transaction(&mut self, tx: BankTransaction) {
//         self.get_or_create_period(PeriodId::from_date(tx.date, self.month_begins_on))
//             .transactions
//             .insert(tx);
//     }
// 
//     pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
//         self.budget_periods
//             .values()
//             .all(|p| p.transactions.can_insert(tx_hash))
//     }
// 
//     pub fn contains_transaction(&self, tx_id: Uuid) -> bool {
//         self.budget_periods
//             .values()
//             .any(|p| p.transactions.contains(tx_id))
//     }
// 
//     pub fn contains_budget_item(&self, item_id: Uuid) -> bool {
//         self.budget_periods.values().any(|p| {
//             p.actual_items
//                 .values()
//                 .any(|i| i.budget_item_id == item_id)
//         })
//     }
// 
//     pub fn contains_item_with_name(&self, name: &str) -> bool {
//         self.budget_periods
//             .values()
//             .any(|p| p.actual_items.values().any(|i| i.item_name() == name))
//     }
// 
//     pub fn get_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BankTransaction> {
//         for period in self.budget_periods.values_mut() {
//             if let Some(tx) = period.transactions.get_mut(tx_id) {
//                 return Some(tx);
//             }
//         }
//         None
//     }
// 
//     pub fn get_transaction(&self, tx_id: Uuid) -> Option<&BankTransaction> {
//         for period in self.budget_periods.values() {
//             if let Some(tx) = period.transactions.get(tx_id) {
//                 return Some(tx);
//             }
//         }
//         None
//     }
// 
//     pub fn get_budgeted_by_type(
//         &self,
//         budgeting_type: &BudgetingType,
//         period_id: PeriodId,
//     ) -> Option<Money> {
//         self.with_period(period_id).map(|p| {
//             p.actual_items
//                 .iter()
//                 .filter(|(_, a)| a.budgeting_type() == *budgeting_type)
//                 .map(|(_, a)| a.budgeted_amount)
//                 .sum()
//         })
//     }
// 
//     pub fn get_actual_by_type(
//         &self,
//         budgeting_type: &BudgetingType,
//         period_id: PeriodId,
//     ) -> Option<Money> {
//         self.with_period(period_id).map(|p| {
//             p.actual_items
//                 .iter()
//                 .filter(|(_, a)| a.budgeting_type() == *budgeting_type)
//                 .map(|(_, a)| a.actual_amount)
//                 .sum()
//         })
//     }
// 
//     pub fn list_bank_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
//         self.with_period(period_id)
//             .map(|p| p.transactions.list_transactions(true))
//             .unwrap_or_default()
//     }
// 
//     pub fn list_transactions_for_actual(
//         &self,
//         period_id: PeriodId,
//         actual_id: Uuid,
//         sorted: bool,
//     ) -> Vec<&BankTransaction> {
//         self.with_period(period_id)
//             .map(|p| p.transactions.list_transactions_for_actual(actual_id, sorted))
//             .unwrap_or_default()
//     }
// 
//     pub fn list_transactions_for_connection(&self, period_id: PeriodId) -> Vec<BankTransaction> {
//         self.with_period(period_id)
//             .map(|p| p.transactions.list_transactions_for_connection())
//             .unwrap_or_default()
//     }
// 
//     pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
//         self.budget_periods
//             .values()
//             .flat_map(|v| v.transactions.list_transactions(true))
//             .collect()
//     }
// 
//     /// Fix up budget_item references in all ActualItems after deserialization
//     /// This replaces the deserialized BudgetItem instances with the shared Arc<Mutex<BudgetItem>>
//     /// references from the Budget's budget_items HashMap
//     pub fn fix_budget_item_references(
//         &mut self,
//         budget_items: &HashMap<Uuid, Arc<Mutex<BudgetItem>>>,
//     ) {
//         for period in self.budget_periods.values_mut() {
//             for actual_item in period.actual_items.values_mut() {
//                 if let Some(shared_budget_item) = budget_items.get(&actual_item.budget_item_id) {
//                     actual_item.budget_item = shared_budget_item.clone();
//                 }
//             }
//         }
//     }
// }
