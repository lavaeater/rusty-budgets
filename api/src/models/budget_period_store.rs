use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use crate::models::{BankTransaction, BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule, Money, MonthBeginsOn, ValueKind};
use crate::models::budget_period::BudgetPeriod;
use crate::models::budget_period_id::BudgetPeriodId;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};

// Custom serialization for HashMap<BudgetPeriodId, BudgetPeriod>
mod budget_period_map_serde {
    use crate::models::budget_period::BudgetPeriod;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use crate::models::budget_period_id::BudgetPeriodId;

    pub fn serialize<S>(
        map: &HashMap<BudgetPeriodId, BudgetPeriod>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, &BudgetPeriod> =
            map.iter().map(|(k, v)| (k.to_string(), v)).collect();
        string_map.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<HashMap<BudgetPeriodId, BudgetPeriod>, D::Error>
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
                Ok((BudgetPeriodId { year, month }, v))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriodStore {
    month_begins_on: MonthBeginsOn,
    current_period_id: BudgetPeriodId,
    #[serde(
        serialize_with = "budget_period_map_serde::serialize",
        deserialize_with = "budget_period_map_serde::deserialize"
    )]
    budget_periods: HashMap<BudgetPeriodId, BudgetPeriod>,
}

impl BudgetPeriodStore {
    pub fn list_ignored_transactions(&self) -> Vec<BankTransaction> {
       self.current_period().transactions.list_ignored_transactions() 
    }
}

impl Default for BudgetPeriodStore {
    fn default() -> Self {
        let month_begins_on = MonthBeginsOn::default();
        let date = Utc::now();
        let id = BudgetPeriodId::from_date(date, month_begins_on);
        Self {
            month_begins_on,
            current_period_id: id,
            budget_periods: HashMap::from([(id, BudgetPeriod::new_for(&id))]),
        }
    }
}

impl BudgetPeriodStore {
    pub fn new(date: DateTime<Utc>, month_begins_on: Option<MonthBeginsOn>) -> Self {
        let month_begins_on = month_begins_on.unwrap_or_default();
        let id = BudgetPeriodId::from_date(date, month_begins_on);
        let period = BudgetPeriod::new_for(&id);
        Self {
            month_begins_on,
            budget_periods: HashMap::from([(id, period.clone())]),
            current_period_id: id,
        }
    }

    pub(crate) fn list_transactions_for_connection(&self) -> Vec<BankTransaction> {
        self.current_period()
            .transactions
            .list_transactions_for_connection()
    }
    pub(crate) fn list_all_items(&self) -> Vec<BudgetItem> {
        self.current_period().budget_items.list_all_items()
    }
    pub(crate) fn current_period_id(&self) -> &BudgetPeriodId {
        &self.current_period_id
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>) -> Vec<(Uuid, Uuid)> {
        self.budget_periods
            .iter()
            .flat_map(|(_, period)| period.evaluate_rules(rules))
            .collect::<Vec<_>>()
    }

    pub(crate) fn move_transaction_to_ignored(&mut self, tx_id: &Uuid) -> bool {
        self.current_period_mut().transactions.ignore_transaction(tx_id)
    }

    pub fn get_period_before(&self, id: &BudgetPeriodId) -> Option<BudgetPeriod> {
        if self.budget_periods.is_empty() {
            return None;
        }
        self.budget_periods
            .keys()
            .filter(|key| key.year < id.year || (key.year == id.year && key.month < id.month))
            .max()
            .map(|key| self.budget_periods.get(key).unwrap().clone())
    }
    
    pub fn set_previous_period(&mut self) {
        let previous_id = self.current_period_id.month_before();
        self.set_current_period_id(&previous_id);
    }

    pub fn set_next_period(&mut self) {
        let next_id = self.current_period_id.month_after();
        self.set_current_period_id(&next_id);
    }

    pub fn get_period_for_date_mut(&mut self, date: &DateTime<Utc>) -> &mut BudgetPeriod {
        self.get_or_create_period(&BudgetPeriodId::from_date(*date, self.month_begins_on))
    }

    pub fn get_period_for_date(&mut self, date: &DateTime<Utc>) -> &BudgetPeriod {
        self.get_period(&BudgetPeriodId::from_date(*date, self.month_begins_on))
    }

    pub fn set_current_period(&mut self, date: &DateTime<Utc>) {
        let period = self.get_period_for_date_mut(date);
        self.current_period_id = period.id;
    }
    
    pub fn set_current_period_id(&mut self, id: &BudgetPeriodId) {
        let _ = self.get_or_create_period(id);
        self.current_period_id = *id;
    }

    pub fn get_or_create_period(&mut self, id: &BudgetPeriodId) -> &mut BudgetPeriod {
        let previous_period = self.get_period_before(id);
        self.budget_periods.entry(*id).or_insert_with(|| {
            if let Some(previous_period) = previous_period {
                let period = previous_period.clone_to(id);
                period.clone()
            } else {
                let period = BudgetPeriod::new_for(id);
                period.clone()
            }
        })
    }

    pub fn get_period(&mut self, id: &BudgetPeriodId) -> &BudgetPeriod {
        self.get_or_create_period(id)
    }

    pub fn current_period(&self) -> &BudgetPeriod {
        tracing::debug!("Current period: {}", self.current_period_id);
        self.budget_periods.get(&self.current_period_id).unwrap()
    }

    pub fn current_period_mut(&mut self) -> &mut BudgetPeriod {
        self.budget_periods
            .get_mut(&self.current_period_id)
            .unwrap()
    }

    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self.current_period().budget_items.get(item_id)
    }

    pub fn get_type_for_item(&self, item_id: &Uuid) -> Option<&BudgetingType> {
        self.current_period().budget_items.type_for(item_id)
    }

    pub fn items_by_type(
        &self,
    ) -> Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)> {
        self.current_period()
            .budget_items
            .items_by_type()
            .iter()
            .map(|(index, t, items)| {
                let overview = self.current_period().budgeting_overview.get(t).unwrap();
                (*index, *t, *overview, items.clone())
            })
            .collect::<Vec<_>>()
    }

    pub fn budgeted_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.current_period()
            .budget_items
            .by_type(budgeting_type)
            .unwrap_or_default()
            .iter()
            .map(|item| item.budgeted_amount)
            .sum()
    }

    pub fn spent_for_type(&self, budgeting_type: &BudgetingType) -> Money {
        self.current_period()
            .budget_items
            .by_type(budgeting_type)
            .unwrap_or_default()
            .iter()
            .map(|item| item.actual_amount)
            .sum()
    }

    pub fn recalc_overview(&mut self) {
        let income_sum = Sum(vec![Income]);
        let budgeted_income = income_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Budgeted),
        );
        let spent_income = income_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Spent),
        );
        let remaining_rule = Difference(Income, vec![Expense, Savings]);
        let remaining_income = remaining_rule.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Budgeted),
        );

        let income_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_income,
            actual_amount: spent_income,
            remaining_budget: remaining_income,
            is_ok: remaining_income > Money::zero(remaining_income.currency())
        };

        self.current_period_mut()
            .budgeting_overview
            .insert(Income, income_overview);

        let expense_sum = Sum(vec![Expense]);
        let budgeted_expenses = expense_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Budgeted),
        );
        let spent_expenses = expense_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Spent),
        );

        let self_difference_rule = SelfDiff(Expense);
        let self_diff =
            self_difference_rule.evaluate(&self.current_period().budget_items.hash_by_type(), None);

        let expense_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_expenses,
            actual_amount: spent_expenses,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency())
        };

        self.current_period_mut()
            .budgeting_overview
            .insert(Expense, expense_overview);

        let savings_sum = Sum(vec![Savings]);
        let budgeted_savings = savings_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Budgeted),
        );
        let spent_savings = savings_sum.evaluate(
            &self.current_period().budget_items.hash_by_type(),
            Some(ValueKind::Spent),
        );

        let self_difference_rule = SelfDiff(Savings);
        let self_diff =
            self_difference_rule.evaluate(&self.current_period().budget_items.hash_by_type(), None);

        let savings_overview = BudgetingTypeOverview {
            budgeted_amount: budgeted_savings,
            actual_amount: spent_savings,
            remaining_budget: self_diff,
            is_ok: self_diff < Money::zero(self_diff.currency())
        };

        self.current_period_mut()
            .budgeting_overview
            .insert(Savings, savings_overview);
    }

    pub fn insert_item(&mut self, item: &BudgetItem, item_type: BudgetingType) {
        self.current_period_mut()
            .budget_items
            .insert(item, item_type);
        self.current_period_mut()
            .budgeted_by_type
            .entry(item_type)
            .and_modify(|v| *v += item.budgeted_amount)
            .or_insert(item.budgeted_amount);
        self.recalc_overview();
    }

    pub fn remove_item(&mut self, item_id: &Uuid) {
        if let Some((item, item_type)) = self.current_period_mut().budget_items.remove(*item_id) {
            self.current_period_mut()
                .budgeted_by_type
                .entry(item_type)
                .and_modify(|v| *v -= item.budgeted_amount);
            self.recalc_overview();
        }
    }

    pub fn insert_transaction(&mut self, tx: BankTransaction) {
        self.get_or_create_period(&BudgetPeriodId::from_date(tx.date, self.month_begins_on))
            .transactions
            .insert(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.budget_periods
            .values()
            .all(|p| p.transactions.can_insert(tx_hash))
    }

    pub fn contains_transaction(&self, tx_id: &Uuid) -> bool {
        self.budget_periods
            .values()
            .any(|p| p.transactions.contains(tx_id))
    }

    pub fn contains_budget_item(&self, item_id: &Uuid) -> bool {
        self.current_period().budget_items.contains(item_id)
    }

    pub fn get_transaction_mut(&mut self, tx_id: &Uuid) -> Option<&mut BankTransaction> {
        self.current_period_mut().transactions.get_mut(tx_id)
    }

    pub fn get_transaction(&self, tx_id: &Uuid) -> Option<&BankTransaction> {
        self.current_period().transactions.get(tx_id)
    }

    pub fn type_for_item(&self, item_id: &Uuid) -> Option<BudgetingType> {
        self.current_period()
            .budget_items
            .type_for(item_id)
            .cloned()
    }

    pub fn update_budget_actual_amount(&mut self, budgeting_type: &BudgetingType, amount: &Money) {
        self.current_period_mut()
            .actual_by_type
            .entry(*budgeting_type)
            .and_modify(|v| {
                *v += *amount;
            });
    }

    pub fn update_budget_budgeted_amount(
        &mut self,
        budgeting_type: &BudgetingType,
        amount: &Money,
    ) {
        self.current_period_mut()
            .budgeted_by_type
            .entry(*budgeting_type)
            .and_modify(|v| {
                *v += *amount;
            });
    }

    pub fn add_actual_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.current_period_mut()
            .budget_items
            .add_actual_amount(item_id, amount);
    }

    pub fn add_budgeted_amount_to_item(&mut self, item_id: &Uuid, amount: &Money) {
        self.current_period_mut()
            .budget_items
            .add_budgeted_amount(item_id, amount);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn modify_budget_item(
        &mut self,
        id: &Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) {
        self.current_period_mut().budget_items.modify_item(
            id,
            name,
            item_type,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        );
    }

    pub fn get_budgeted_by_type(&self, budgeting_type: &BudgetingType) -> Option<&Money> {
        self.current_period().budgeted_by_type.get(budgeting_type)
    }

    pub fn get_actual_by_type(&self, budgeting_type: &BudgetingType) -> Option<&Money> {
        self.current_period().actual_by_type.get(budgeting_type)
    }

    pub fn get_budgeting_overview(
        &self,
        budgeting_type: &BudgetingType,
    ) -> Option<&BudgetingTypeOverview> {
        self.current_period().budgeting_overview.get(budgeting_type)
    }

    pub fn list_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.current_period().transactions.list_transactions(true)
    }
    pub fn list_transactions_for_item(&self, item_id: &Uuid, sorted: bool) -> Vec<&BankTransaction> {
        self.current_period().transactions.list_transactions_for_item(item_id, sorted)
    }

    pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods
            .values()
            .flat_map(|v| v.transactions.list_transactions(true))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    
}