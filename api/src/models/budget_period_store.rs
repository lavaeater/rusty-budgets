use crate::models::budget_period::BudgetPeriod;
use crate::models::budget_period_id::PeriodId;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};
use crate::models::{
    ActualItem, BankTransaction, BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule,
    Money, MonthBeginsOn, Rule, ValueKind,
};
use anyhow::__private::NotBothDebug;
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use iter_tools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Custom serialization for HashMap<BudgetPeriodId, BudgetPeriod>
mod budget_period_map_serde {
    use crate::models::budget_period::BudgetPeriod;
    use crate::models::budget_period_id::PeriodId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<PeriodId, BudgetPeriod>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, &BudgetPeriod> =
            map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<PeriodId, BudgetPeriod>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_map: HashMap<String, BudgetPeriod> = HashMap::deserialize(deserializer)?;
        string_map
            .into_iter()
            .map(|(k, v)| {
                let parts: Vec<&str> = k.split('-').collect();
                if parts.len() != 2 {
                    return Err(serde::de::Error::custom(format!(
                        "Invalid BudgetPeriodId format: {}",
                        k
                    )));
                }
                let year = parts[0]
                    .parse::<i32>()
                    .map_err(|e| serde::de::Error::custom(format!("Invalid year: {}", e)))?;
                let month = parts[1]
                    .parse::<u32>()
                    .map_err(|e| serde::de::Error::custom(format!("Invalid month: {}", e)))?;
                Ok((PeriodId { year, month }, v))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriodStore {
    month_begins_on: MonthBeginsOn,
    #[serde(
        serialize_with = "budget_period_map_serde::serialize",
        deserialize_with = "budget_period_map_serde::deserialize"
    )]
    budget_periods: HashMap<PeriodId, BudgetPeriod>,
    #[serde(default)]
    rules: RulePackages,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackages {
    pub rule_packages: Vec<RulePackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackage {
    pub budgeting_type: BudgetingType,
    pub budgeted_rule: Rule,
    pub spent_rule: Rule,
    pub remaining_rule: Rule,
}

impl RulePackage {
    pub fn new(
        budgeting_type: BudgetingType,
        budgeted_rule: Rule,
        actual_rule: Rule,
        remaining_rule: Rule,
    ) -> Self {
        Self {
            budgeting_type,
            budgeted_rule,
            spent_rule: actual_rule,
            remaining_rule,
        }
    }
}

impl Default for RulePackages {
    fn default() -> Self {
        Self {
            rule_packages: vec![
                RulePackage::new(
                    Income,
                    Sum(vec![Income]),
                    Sum(vec![Income]),
                    Difference(Income, vec![Expense, Savings]),
                ),
                RulePackage::new(
                    Expense,
                    Sum(vec![Expense]),
                    Sum(vec![Expense]),
                    SelfDiff(Expense),
                ),
                RulePackage::new(
                    Savings,
                    Sum(vec![Savings]),
                    Sum(vec![Savings]),
                    SelfDiff(Savings),
                ),
            ],
        }
    }
}

impl Default for BudgetPeriodStore {
    fn default() -> Self {
        let month_begins_on = MonthBeginsOn::default();
        let date = Utc::now();
        let id = PeriodId::from_date(date, month_begins_on);
        Self {
            month_begins_on,
            budget_periods: HashMap::from([(id, BudgetPeriod::new_for(id))]),
            rules: Default::default(),
        }
    }
}

impl BudgetPeriodStore {
    pub fn get_period_for_transaction(&self, tx_id: Uuid) -> Option<&BudgetPeriod> {
        self.budget_periods
            .values()
            .find(|p| p.transactions.contains(tx_id))
    }

    pub fn new(date: DateTime<Utc>, month_begins_on: Option<MonthBeginsOn>) -> Self {
        let month_begins_on = month_begins_on.unwrap_or_default();
        let id = PeriodId::from_date(date, month_begins_on);
        let period = BudgetPeriod::new_for(id);
        Self {
            month_begins_on,
            budget_periods: HashMap::from([(id, period.clone())]),
            rules: Default::default(),
        }
    }

    pub fn ensure_period(&mut self, id: PeriodId) {
        self.get_or_create_period(id);
    }

    pub fn with_period(&self, id: PeriodId) -> Option<&BudgetPeriod> {
        self.budget_periods.get(&id)
    }

    pub fn with_period_mut(&mut self, id: PeriodId) -> Option<&mut BudgetPeriod> {
        self.budget_periods.get_mut(&id)
    }

    pub fn list_ignored_transactions(&self, period_id: PeriodId) -> Vec<BankTransaction> {
        self.with_period(period_id)
            .map(|p| p.transactions.list_ignored_transactions())
            .unwrap_or_default()
    }

    pub(crate) fn month_begins_on(&self) -> MonthBeginsOn {
        self.month_begins_on
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>) -> Vec<(Uuid, Uuid)> {
        self.budget_periods
            .iter()
            .flat_map(|(_, period)| period.evaluate_rules(rules))
            .collect::<Vec<_>>()
    }

    pub fn move_transaction_to_ignored(&mut self, tx_id: Uuid, period_id: PeriodId) -> bool {
        self.with_period_mut(period_id)
            .map(|p| p.transactions.ignore_transaction(tx_id))
            .unwrap_or_default()
    }

    pub fn get_period_before(&self, id: PeriodId) -> Option<&BudgetPeriod> {
        if self.budget_periods.is_empty() {
            return None;
        }
        self.budget_periods
            .keys()
            .filter(|key| key.year < id.year || (key.year == id.year && key.month < id.month))
            .max()
            .map(|key| self.budget_periods.get(key).unwrap())
    }

    pub fn create_period_before(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period = period_id.month_before();
        self.get_or_create_period(period)
    }

    pub fn create_period_after(&mut self, period_id: PeriodId) -> &mut BudgetPeriod {
        let period = period_id.month_after();
        self.get_or_create_period(period)
    }

    pub fn get_period_for_date_mut(&mut self, date: &DateTime<Utc>) -> &mut BudgetPeriod {
        self.get_or_create_period(PeriodId::from_date(*date, self.month_begins_on))
    }

    pub fn get_period_for_date(&mut self, date: &DateTime<Utc>) -> &BudgetPeriod {
        self.get_or_create_period(PeriodId::from_date(*date, self.month_begins_on))
    }

    fn get_or_create_period(&mut self, id: PeriodId) -> &mut BudgetPeriod {
        if self.budget_periods.contains_key(&id) {
            return self.budget_periods.get_mut(&id).unwrap();
        }
        let previous_period = self.get_period_before(id);
        let period = if let Some(previous_period) = previous_period {
            previous_period.clone_to(id)
        } else {
            BudgetPeriod::new_for(id)
        };
        self.budget_periods.insert(id, period);
        self.budget_periods.get_mut(&id).unwrap()
    }

    pub fn items_by_type(
        &self,
        period_id: PeriodId,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<ActualItem>)> {
        let b = self
            .with_period(period_id)
            .map(|p| {
                p.actual_items
                    .values()
                    .group_by(|item| item.budgeting_type())
                    .into_iter()
                    .enumerate()
                    .map(|(index, (group, items))| {
                        let overview = match group {
                            Income => self.get_income_overview(period_id),
                            Expense => self.get_expense_overview(period_id),
                            Savings => self.get_savings_overview(period_id),
                        };
                        (index, group, overview, items.cloned().collect::<Vec<_>>())
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        b
    }

    pub fn list_items_of_type(
        &self,
        budgeting_type: BudgetingType,
        period_id: PeriodId,
    ) -> Vec<&ActualItem> {
        self.with_period(period_id)
            .map(|p| {
                p.actual_items
                    .values()
                    .filter(|item| item.budgeting_type() == budgeting_type)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    pub fn budgeted_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.list_items_of_type(budgeting_type, period_id)
            .iter()
            .map(|item| item.budgeted_amount)
            .sum()
    }

    pub fn spent_for_type(&self, budgeting_type: BudgetingType, period_id: PeriodId) -> Money {
        self.list_items_of_type(budgeting_type, period_id)
            .iter()
            .map(|item| item.actual_amount)
            .sum()
    }

    pub fn get_income_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
        let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Income).unwrap();
        let items = &self.list_items_of_type(Income, period_id);
        let budgeted_income = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
        let spent_income = rules.spent_rule.evaluate(items, Some(ValueKind::Spent));
        let remaining_income = rules.remaining_rule.evaluate(items, None);

        let income_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_income,
            actual_amount: spent_income,
            remaining_budget: remaining_income,
            is_ok: remaining_income == Money::zero(remaining_income.currency()),
        };
        income_overview
    }

    pub fn get_expense_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
        let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Expense).unwrap();
        let items = &self.list_items_of_type(Expense, period_id);
        let budgeted_expenses = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
        let spent_expenses = rules.spent_rule.evaluate(items, Some(ValueKind::Spent));
        let self_diff = rules.remaining_rule.evaluate(items, None);

        let expense_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_expenses,
            actual_amount: spent_expenses,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency()),
        };
        expense_overview
    }

    pub fn get_savings_overview(&self, period_id: PeriodId) -> BudgetingTypeOverview {
        let rules = &self.rules.rule_packages.iter().find(|p| p.budgeting_type == Savings).unwrap();
        let items = &self.list_items_of_type(Savings, period_id);
        let budgeted_savings = rules.budgeted_rule.evaluate(items, Some(ValueKind::Budgeted));
        let spent_savings = rules.spent_rule.evaluate(items, Some(ValueKind::Spent));
        let self_diff = rules.remaining_rule.evaluate(items, None);

        let savings_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_savings,
            actual_amount: spent_savings,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency()),
        };
        savings_overview
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) {
        self.get_or_create_period(PeriodId::from_date(tx.date, self.month_begins_on))
            .transactions
            .insert(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.budget_periods
            .values()
            .all(|p| p.transactions.can_insert(tx_hash))
    }

    pub fn contains_transaction(&self, tx_id: Uuid) -> bool {
        self.budget_periods
            .values()
            .any(|p| p.transactions.contains(tx_id))
    }

    pub fn contains_budget_item(&self, item_id: Uuid) -> bool {
        self.budget_periods.values().any(|p| {
            p.actual_items
                .values()
                .any(|i| i.budget_item_id == item_id)
        })
    }

    pub fn contains_item_with_name(&self, name: &str) -> bool {
        self.budget_periods
            .values()
            .any(|p| p.actual_items.values().any(|i| i.item_name() == name))
    }

    pub fn get_transaction_mut(&mut self, tx_id: Uuid) -> Option<&mut BankTransaction> {
        for period in self.budget_periods.values_mut() {
            if let Some(tx) = period.transactions.get_mut(tx_id) {
                return Some(tx);
            }
        }
        None
    }

    pub fn get_transaction(&self, tx_id: Uuid) -> Option<&BankTransaction> {
        for period in self.budget_periods.values() {
            if let Some(tx) = period.transactions.get(tx_id) {
                return Some(tx);
            }
        }
        None
    }

    pub fn get_budgeted_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Option<Money> {
        self.with_period(period_id).map(|p| {
            p.actual_items
                .iter()
                .filter(|(_, a)| a.budgeting_type() == *budgeting_type)
                .map(|(_, a)| a.budgeted_amount)
                .sum()
        })
    }

    pub fn get_actual_by_type(
        &self,
        budgeting_type: &BudgetingType,
        period_id: PeriodId,
    ) -> Option<Money> {
        self.with_period(period_id).map(|p| {
            p.actual_items
                .iter()
                .filter(|(_, a)| a.budgeting_type() == *budgeting_type)
                .map(|(_, a)| a.actual_amount)
                .sum()
        })
    }

    pub fn list_bank_transactions(&self, period_id: PeriodId) -> Vec<&BankTransaction> {
        self.with_period(period_id)
            .map(|p| p.transactions.list_transactions(true))
            .unwrap_or_default()
    }

    pub fn list_transactions_for_item(
        &self,
        period_id: PeriodId,
        item_id: Uuid,
        sorted: bool,
    ) -> Vec<&BankTransaction> {
        self.with_period(period_id)
            .map(|p| p.transactions.list_transactions_for_item(item_id, sorted))
            .unwrap_or_default()
    }

    pub fn list_transactions_for_connection(&self, period_id: PeriodId) -> Vec<BankTransaction> {
        self.with_period(period_id)
            .map(|p| p.transactions.list_transactions_for_connection())
            .unwrap_or_default()
    }

    pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods
            .values()
            .flat_map(|v| v.transactions.list_transactions(true))
            .collect()
    }

    /// Fix up budget_item references in all ActualItems after deserialization
    /// This replaces the deserialized BudgetItem instances with the shared Arc<Mutex<BudgetItem>>
    /// references from the Budget's budget_items HashMap
    pub fn fix_budget_item_references(
        &mut self,
        budget_items: &HashMap<Uuid, Arc<Mutex<BudgetItem>>>,
    ) {
        for period in self.budget_periods.values_mut() {
            for actual_item in period.actual_items.values_mut() {
                if let Some(shared_budget_item) = budget_items.get(&actual_item.budget_item_id) {
                    actual_item.budget_item = shared_budget_item.clone();
                }
            }
        }
    }
}
