use crate::models::{BankTransactionStore, BudgetItem, BudgetItemStore, BudgetingType, BudgetingTypeOverview, Money};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetPeriodStore {
    pub current_period_id: Option<BudgetPeriodId>,
    pub budget_periods: HashMap<BudgetPeriodId, BudgetPeriod>,
}

impl BudgetPeriodStore {
    pub fn new(year: i32, month: u32) -> Self {
        let id = BudgetPeriodId::from(year, month);
        let period = BudgetPeriod::new_for(&id);
        Self {
            budget_periods: HashMap::from([(id, period.clone())]),
            current_period_id: Some(id),
        }
    }

    pub fn get_period_before(&self, year: i32, month: u32) -> Option<BudgetPeriod> {
        if self.budget_periods.is_empty() {
            return None;
        }
        let id = BudgetPeriodId { year, month };

        self.budget_periods
            .keys()
            .filter(|key| key.year < id.year || (key.year == id.year && key.month < id.month))
            .max()
            .map(|key| self.budget_periods.get(key).unwrap().clone())
    }

    pub fn get_for_date(&mut self, date: DateTime<Utc>) -> &mut BudgetPeriod {
        let year = date.year();
        let month = date.month();
        self.get_period(year, month)
    }

    pub fn get_period(&mut self, year: i32, month: u32) -> &mut BudgetPeriod {
        let id = BudgetPeriodId { year, month };
        let previous_period = self.get_period_before(year, month);
        self.budget_periods.entry(id).or_insert_with(|| {
            if let Some(previous_period) = previous_period {
                let period = previous_period.clone_to(&id);
                period.clone()
            } else {
                let period = BudgetPeriod::new_for(&id);
                period.clone()
            }
        });
        self.budget_periods.get_mut(&id).unwrap()
    }
}

#[derive(Copy, Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct BudgetPeriodId {
    pub year: i32,
    pub month: u32,
}

impl BudgetPeriodId {
    pub fn from(year: i32, month: u32) -> Self {
        Self { year, month }
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
            (BudgetingType::Expense, BudgetingTypeOverview::default()),
            (BudgetingType::Savings, BudgetingTypeOverview::default()),
            (BudgetingType::Income, BudgetingTypeOverview::default()),
        ]);
        self.budgeted_by_type = HashMap::from([
            (BudgetingType::Expense, Money::default()),
            (BudgetingType::Savings, Money::default()),
            (BudgetingType::Income, Money::default()),
        ]);
        self.actual_by_type = HashMap::from([
            (BudgetingType::Expense, Money::default()),
            (BudgetingType::Savings, Money::default()),
            (BudgetingType::Income, Money::default()),
        ]);
    }
    fn clone_to(&self, id: &BudgetPeriodId) -> Self {
        let mut period = self.clone();
        period.id = *id;
        period.clear_hashmaps_and_transactions();
        period
    }
    fn new_for(id: &BudgetPeriodId) -> Self {
        Self {
            id: *id,
            budget_items: BudgetItemStore::default(),
            bank_transactions: BankTransactionStore::default(),
            budgeted_by_type: Default::default(),
            actual_by_type: Default::default(),
            budgeting_overview: Default::default(),
        }
    }
    pub fn new_for_year_and_month(year: i32, month: u32) -> Self {
        let id = BudgetPeriodId::from(year, month);
        Self::new_for(&id)
    }
}
