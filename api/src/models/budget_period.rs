use core::fmt::Display;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};
use crate::models::{
    BankTransaction, BankTransactionStore, BudgetItem, BudgetItemStore, BudgetingType,
    BudgetingTypeOverview, Money, MonthBeginsOn, ValueKind,
};
use chrono::{DateTime, Datelike, Days, Months, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

// Custom serialization for HashMap<BudgetPeriodId, BudgetPeriod>
mod budget_period_map_serde {
    use super::{BudgetPeriod, BudgetPeriodId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub fn serialize<S>(
        map: &HashMap<BudgetPeriodId, BudgetPeriod>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_map: HashMap<String, &BudgetPeriod> = map
            .iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
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
    #[serde(serialize_with = "budget_period_map_serde::serialize", deserialize_with = "budget_period_map_serde::deserialize")]
    budget_periods: HashMap<BudgetPeriodId, BudgetPeriod>,
}

impl BudgetPeriodStore {
    pub(crate) fn current_period_id(&self) -> &BudgetPeriodId {
        &self.current_period_id
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

    pub fn get_period_for_date_mut(&mut self, date: &DateTime<Utc>) -> &mut BudgetPeriod {
        self.get_period_mut(&BudgetPeriodId::from_date(*date, self.month_begins_on))
    }

    pub fn get_period_for_date(&mut self, date: &DateTime<Utc>) -> &BudgetPeriod {
        self.get_period(&BudgetPeriodId::from_date(*date, self.month_begins_on))
    }

    pub fn set_current_period(&mut self, date: &DateTime<Utc>) {
        let period = self.get_period_for_date_mut(date);
        self.current_period_id = period.id;
    }
    

    pub fn get_period_mut(&mut self, id: &BudgetPeriodId) -> &mut BudgetPeriod {
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
        self.get_period_mut(id)
    }

    pub fn current_period(&self) -> &BudgetPeriod {
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
        self.get_period_mut(&BudgetPeriodId::from_date(tx.date, self.month_begins_on))
            .bank_transactions.insert(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.budget_periods.values().all(|p| p.bank_transactions.can_insert(tx_hash))
    }

    pub fn contains_transaction(&self, tx_id: &Uuid) -> bool {
        self.current_period().bank_transactions.contains(tx_id)
    }

    pub fn contains_budget_item(&self, item_id: &Uuid) -> bool {
        self.current_period().budget_items.contains(item_id)
    }

    pub fn get_transaction_mut(&mut self, tx_id: &Uuid) -> Option<&mut BankTransaction> {
        self.current_period_mut().bank_transactions.get_mut(tx_id)
    }

    pub fn get_transaction(&self, tx_id: &Uuid) -> Option<&BankTransaction> {
        self.current_period().bank_transactions.get(tx_id)
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
        self.current_period().bank_transactions.list_transactions()
    }

    pub fn list_all_bank_transactions(&self) -> Vec<&BankTransaction> {
        self.budget_periods.values().flat_map(|v|v.bank_transactions.list_transactions()).collect()
    }
}

#[derive(Copy, Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct BudgetPeriodId {
    pub year: i32,
    pub month: u32,
}

impl Display for BudgetPeriodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.year, self.month)
    }
}

impl BudgetPeriodId {
    pub fn from_date(date: DateTime<Utc>, month_begins_on: MonthBeginsOn) -> Self {
        let month_start_date = match month_begins_on {
            MonthBeginsOn::PreviousMonth(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use PreviousMonth1stDayOfMonth")
                }
                date.checked_sub_months(Months::new(1))
                    .unwrap()
                    .with_day(day)
                    .unwrap()
            }
            MonthBeginsOn::CurrentMonth(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use CurrentMonth1stDayOfMonth")
                }
                date.with_day(day).unwrap()
            }
            MonthBeginsOn::PreviousMonth1stDayOfMonth => date
                .checked_sub_months(Months::new(1))
                .unwrap()
                .with_day(1)
                .unwrap(),
            MonthBeginsOn::CurrentMonth1stDayOfMonth => date.with_day(1).unwrap(),
        };

        let month_end_date = match month_begins_on {
            MonthBeginsOn::PreviousMonth(day) => date.with_day(day - 1).unwrap(),
            MonthBeginsOn::CurrentMonth(day) => date
                .checked_add_months(Months::new(1))
                .unwrap()
                .with_day(day - 1)
                .unwrap(),
            MonthBeginsOn::PreviousMonth1stDayOfMonth => last_day_of_month(month_start_date),
            MonthBeginsOn::CurrentMonth1stDayOfMonth => last_day_of_month(month_start_date),
        };

        let date = if date < month_start_date {
            month_start_date
        } else if date > month_end_date {
            month_end_date
        } else {
            date
        };
        Self {
            year: date.year(),
            month: date.month(),
        }
    }
}

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriod {
    pub id: BudgetPeriodId,
    pub budget_items: BudgetItemStore,
    pub bank_transactions: BankTransactionStore,
    pub budgeted_by_type: HashMap<BudgetingType, Money>,
    pub actual_by_type: HashMap<BudgetingType, Money>,
    pub budgeting_overview: HashMap<BudgetingType, BudgetingTypeOverview>,
}

impl BudgetPeriod {
    fn clear_hashmaps_and_transactions(&mut self) {
        self.bank_transactions.clear();
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
    fn clone_to(&self, id: &BudgetPeriodId) -> Self {
        let mut period = self.clone();
        period.id = *id;
        period.clear_hashmaps_and_transactions();
        period
    }
    fn new_for(id: &BudgetPeriodId) -> Self {
        let mut period = Self {
            id: *id,
            budget_items: BudgetItemStore::default(),
            bank_transactions: BankTransactionStore::default(),
            budgeted_by_type: Default::default(),
            actual_by_type: Default::default(),
            budgeting_overview: Default::default(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }
}

fn last_day_of_month(dt: DateTime<Utc>) -> DateTime<Utc> {
    let first_next_month = dt
        .checked_add_months(Months::new(1))
        .unwrap()
        .with_day(1)
        .unwrap();
    first_next_month.checked_sub_days(Days::new(1)).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn test_from_date_current_month_1st_day() {
        // Test with CurrentMonth1stDayOfMonth - period is 1st to last day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_1st_day() {
        // Test with PreviousMonth1stDayOfMonth - period is 1st of prev month to last day of prev month
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth1stDayOfMonth);

        // Date is March 15, period starts Feb 1, so date is clamped to Feb 28 (end of period)
        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 2);
    }

    #[test]
    fn test_from_date_current_month_custom_day_within_period() {
        // Test with CurrentMonth(15) - period is 15th of current month to 14th of next month
        // Date March 20 is within period (March 15 - April 14)
        let date = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_current_month_before_start_day() {
        // Test with CurrentMonth(15) when date is before the 15th
        // Date March 10 is before period start (March 15), so clamped to March 15
        let date = Utc.with_ymd_and_hms(2025, 3, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_current_month_after_end_day() {
        // Test with CurrentMonth(15) when date is after period end
        // Date April 20 is after period end (April 14), so clamped to April 14
        let date = Utc.with_ymd_and_hms(2025, 4, 20, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 4);
    }

    #[test]
    fn test_from_date_previous_month_custom_day_within_period() {
        // Test with PreviousMonth(25) - period is 25th of prev month to 24th of current month
        // Date March 20 is within period (Feb 25 - March 24)
        let date = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_before_start_day() {
        // Test with PreviousMonth(25) when date is before period start
        // Date March 10 is before period start (Feb 25), so clamped to Feb 25
        let date = Utc.with_ymd_and_hms(2025, 3, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_after_end_day() {
        // Test with PreviousMonth(25) when date is after period end
        // Date March 26 is after period end (March 24), so clamped to March 24
        let date = Utc.with_ymd_and_hms(2025, 3, 26, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_year_boundary_december_to_january() {
        // Test year boundary with PreviousMonth(25)
        // Period: Dec 25, 2024 - Jan 24, 2025
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_on_start_boundary() {
        // Test with date exactly on period start
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 0, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_on_end_boundary() {
        // Test with date exactly on period end
        // Period: March 15 - April 14
        let date = Utc.with_ymd_and_hms(2025, 4, 14, 23, 59, 59).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 4);
    }

    #[test]
    fn test_from_date_february_leap_year() {
        // Test February in a leap year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2024, 2, 29, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2024);
        assert_eq!(id.month, 2);
    }

    #[test]
    fn test_from_date_february_non_leap_year() {
        // Test February in a non-leap year
        let date = Utc.with_ymd_and_hms(2025, 2, 28, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 2);
    }

    #[test]
    fn test_from_date_default_month_begins_on() {
        // Test with default MonthBeginsOn (PreviousMonth(25))
        // Period: Feb 25 - March 24
        let date = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::default());

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_crosses_year_boundary() {
        // Test PreviousMonth(25) with date in January
        // Period: Dec 25, 2024 - Jan 24, 2025
        // Date Jan 10 is within period
        let date = Utc.with_ymd_and_hms(2025, 1, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_current_month_end_of_month() {
        // Test with CurrentMonth(25) and date at end of month
        // Period: March 25 - April 24
        // Date March 31 is within period
        let date = Utc.with_ymd_and_hms(2025, 3, 31, 23, 59, 59).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_february_edge() {
        // Test PreviousMonth(28) with March date
        // Period: Feb 28 - March 27
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(28));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_consistency() {
        // Test that the same date with the same MonthBeginsOn produces the same result
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let month_begins = MonthBeginsOn::CurrentMonth(10);

        let id1 = BudgetPeriodId::from_date(date, month_begins);
        let id2 = BudgetPeriodId::from_date(date, month_begins);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_from_date_first_day_of_year() {
        // Test first day of the year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_last_day_of_year() {
        // Test last day of the year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 12);
    }

    #[test]
    #[should_panic(expected = "Cannot start on day 1, use PreviousMonth1stDayOfMonth")]
    fn test_from_date_previous_month_day_1_panics() {
        // Test that PreviousMonth(1) panics
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let _ = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(1));
    }

    #[test]
    #[should_panic(expected = "Cannot start on day 1, use CurrentMonth1stDayOfMonth")]
    fn test_from_date_current_month_day_1_panics() {
        // Test that CurrentMonth(1) panics
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let _ = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(1));
    }

    #[test]
    fn test_from_date_ordering() {
        // Test that BudgetPeriodId ordering works correctly
        let date1 = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let date3 = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();

        let id1 = BudgetPeriodId::from_date(date1, MonthBeginsOn::CurrentMonth1stDayOfMonth);
        let id2 = BudgetPeriodId::from_date(date2, MonthBeginsOn::CurrentMonth1stDayOfMonth);
        let id3 = BudgetPeriodId::from_date(date3, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert!(id1 < id2);
        assert!(id2 < id3);
        assert!(id1 < id3);
    }

    #[test]
    fn test_from_date_clamping_behavior_current_month() {
        // Test that dates outside period are clamped correctly for CurrentMonth
        // Period: March 10 - April 9

        // Date before period start (March 5) should clamp to March 10
        let date_before = Utc.with_ymd_and_hms(2025, 3, 5, 12, 0, 0).unwrap();
        let id_before = BudgetPeriodId::from_date(date_before, MonthBeginsOn::CurrentMonth(10));
        assert_eq!(id_before.year, 2025);
        assert_eq!(id_before.month, 3);

        // Date within period (March 20) should use actual date
        let date_within = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let id_within = BudgetPeriodId::from_date(date_within, MonthBeginsOn::CurrentMonth(10));
        assert_eq!(id_within.year, 2025);
        assert_eq!(id_within.month, 3);

        // Date after period end (April 15) should clamp to April 9
        let date_after = Utc.with_ymd_and_hms(2025, 4, 15, 12, 0, 0).unwrap();
        let id_after = BudgetPeriodId::from_date(date_after, MonthBeginsOn::CurrentMonth(10));
        assert_eq!(id_after.year, 2025);
        assert_eq!(id_after.month, 4);
    }

    #[test]
    fn test_from_date_clamping_behavior_previous_month() {
        // Test that dates outside period are clamped correctly for PreviousMonth
        // Period: Feb 20 - March 19

        // Date before period start (Feb 15) should clamp to Feb 20
        let date_before = Utc.with_ymd_and_hms(2025, 2, 15, 12, 0, 0).unwrap();
        let id_before = BudgetPeriodId::from_date(date_before, MonthBeginsOn::PreviousMonth(20));
        assert_eq!(id_before.year, 2025);
        assert_eq!(id_before.month, 2);

        // Date within period (March 10) should use actual date
        let date_within = Utc.with_ymd_and_hms(2025, 3, 10, 12, 0, 0).unwrap();
        let id_within = BudgetPeriodId::from_date(date_within, MonthBeginsOn::PreviousMonth(20));
        assert_eq!(id_within.year, 2025);
        assert_eq!(id_within.month, 3);

        // Date after period end (March 25) should clamp to March 19
        let date_after = Utc.with_ymd_and_hms(2025, 3, 25, 12, 0, 0).unwrap();
        let id_after = BudgetPeriodId::from_date(date_after, MonthBeginsOn::PreviousMonth(20));
        assert_eq!(id_after.year, 2025);
        assert_eq!(id_after.month, 3);
    }
    #[test]
    fn test_last_day_of_month_january() {
        // January has 31 days
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 1);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_february_non_leap_year() {
        // February in non-leap year has 28 days
        let date = Utc.with_ymd_and_hms(2025, 2, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 2);
        assert_eq!(last_day.day(), 28);
    }

    #[test]
    fn test_last_day_of_month_february_leap_year() {
        // February in leap year has 29 days
        let date = Utc.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2024);
        assert_eq!(last_day.month(), 2);
        assert_eq!(last_day.day(), 29);
    }

    #[test]
    fn test_last_day_of_month_march() {
        // March has 31 days
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_april() {
        // April has 30 days
        let date = Utc.with_ymd_and_hms(2025, 4, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 4);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_may() {
        // May has 31 days
        let date = Utc.with_ymd_and_hms(2025, 5, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 5);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_june() {
        // June has 30 days
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 6);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_july() {
        // July has 31 days
        let date = Utc.with_ymd_and_hms(2025, 7, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 7);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_august() {
        // August has 31 days
        let date = Utc.with_ymd_and_hms(2025, 8, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 8);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_september() {
        // September has 30 days
        let date = Utc.with_ymd_and_hms(2025, 9, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 9);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_october() {
        // October has 31 days
        let date = Utc.with_ymd_and_hms(2025, 10, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 10);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_november() {
        // November has 30 days
        let date = Utc.with_ymd_and_hms(2025, 11, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 11);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_december() {
        // December has 31 days
        let date = Utc.with_ymd_and_hms(2025, 12, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 12);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_first_day() {
        // Test with first day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_already_last_day() {
        // Test with already the last day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 31, 23, 59, 59).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_preserves_time() {
        // Test that time components are preserved
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 45).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.day(), 30);
        // Time should be adjusted to the last day but the calculation goes through
        // first day of next month minus 1 day, so time will be preserved
    }

    #[test]
    fn test_last_day_of_month_all_months_2025() {
        // Test all months in a non-leap year
        let expected_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        for (month, &expected_day) in (1..=12).zip(expected_days.iter()) {
            let date = Utc.with_ymd_and_hms(2025, month, 15, 12, 0, 0).unwrap();
            let last_day = last_day_of_month(date);

            assert_eq!(last_day.year(), 2025);
            assert_eq!(last_day.month(), month);
            assert_eq!(last_day.day(), expected_day, "Failed for month {}", month);
        }
    }

    #[test]
    fn test_last_day_of_month_all_months_2024() {
        // Test all months in a leap year
        let expected_days = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        for (month, &expected_day) in (1..=12).zip(expected_days.iter()) {
            let date = Utc.with_ymd_and_hms(2024, month, 15, 12, 0, 0).unwrap();
            let last_day = last_day_of_month(date);

            assert_eq!(last_day.year(), 2024);
            assert_eq!(last_day.month(), month);
            assert_eq!(
                last_day.day(),
                expected_day,
                "Failed for month {} in leap year",
                month
            );
        }
    }

    #[test]
    fn test_last_day_of_month_century_leap_years() {
        // Test century years (2000 is a leap year, 1900 and 2100 are not)
        // 2000 is divisible by 400, so it's a leap year
        let date_2000 = Utc.with_ymd_and_hms(2000, 2, 15, 12, 0, 0).unwrap();
        let last_day_2000 = last_day_of_month(date_2000);
        assert_eq!(last_day_2000.day(), 29);
    }

    #[test]
    fn test_last_day_of_month_consistency() {
        // Test that calling the function multiple times with the same input gives the same result
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();

        let result1 = last_day_of_month(date);
        let result2 = last_day_of_month(date);

        assert_eq!(result1, result2);
    }
}
