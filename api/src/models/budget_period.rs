use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::Rule::{Difference, SelfDiff, Sum};
use crate::models::{
    BankTransaction, BankTransactionStore, BudgetItem, BudgetItemStore, BudgetingType,
    BudgetingTypeOverview, Money, ValueKind,
};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPeriodStore {
    current_period_id: BudgetPeriodId,
    budget_periods: HashMap<BudgetPeriodId, BudgetPeriod>,
}

impl Default for BudgetPeriodStore {
    fn default() -> Self {
        let date = Utc::now();
        let id = BudgetPeriodId::from(date.year(), date.month());
        Self {
            current_period_id: id,
            budget_periods: HashMap::from([(id, BudgetPeriod::new_for(&id))]),
        }
    }
}

impl BudgetPeriodStore {
    pub fn new(year: i32, month: u32) -> Self {
        let id = BudgetPeriodId::from(year, month);
        let period = BudgetPeriod::new_for(&id);
        Self {
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

    pub fn get_for_date(&mut self, date: DateTime<Utc>) -> &mut BudgetPeriod {
        self.get_period_mut(&BudgetPeriodId::from_date(date))
    }
    
    pub fn set_current_period(&mut self, date: DateTime<Utc>) {
        let period = self.get_for_date(date);
        self.current_period_id = period.id;
    }

    pub fn get_period_mut(&mut self, id: &BudgetPeriodId) -> &mut BudgetPeriod {
        let previous_period = self.get_period_before(id);
        self.budget_periods.entry(*id).or_insert_with(|| {
            if let Some(previous_period) = previous_period {
                let period = previous_period.clone_to(&id);
                period.clone()
            } else {
                let period = BudgetPeriod::new_for(&id);
                period.clone()
            }
        })
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
        self.current_period_mut().bank_transactions.insert(tx);
    }

    pub fn can_insert_transaction(&self, tx_hash: &u64) -> bool {
        self.current_period().bank_transactions.can_insert(tx_hash)
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
    
    pub fn from_date(date: DateTime<Utc>) -> Self {
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
    pub fn new_for_year_and_month(year: i32, month: u32) -> Self {
        let id = BudgetPeriodId::from(year, month);
        Self::new_for(&id)
    }
}
