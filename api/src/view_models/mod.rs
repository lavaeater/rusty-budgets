use chrono::{DateTime, Utc};
use crate::models::{ActualItem, BankTransaction, Budget, BudgetItem, BudgetingType, Currency, Money, MonthBeginsOn, PeriodId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetItemViewModel {
    pub item_id: Uuid,
    pub actual_id: Option<Uuid>,
    pub name: String,
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
}

impl BudgetItemViewModel {
    pub fn from_item(budget_item: &BudgetItem, actual_item: Option<&ActualItem>, currency: Currency) -> Self {
        if let Some(actual_item) = actual_item {
            Self {
                item_id: actual_item.budget_item_id,
                actual_id: Some(actual_item.id),
                name: actual_item.item_name().clone(),
                budgeting_type: actual_item.budgeting_type(),
                budgeted_amount: actual_item.budgeted_amount,
                actual_amount: actual_item.actual_amount,
                remaining_budget: actual_item.budgeted_amount - actual_item.actual_amount,
            }   
        } else {
            Self {
                item_id: budget_item.id,
                actual_id: None,
                name: budget_item.name.clone(),
                budgeting_type: budget_item.budgeting_type,
                budgeted_amount: Money::zero(currency),
                actual_amount: Money::zero(currency),
                remaining_budget: Money::zero(currency),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransactionViewModel {
    pub tx_id: Uuid,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub actual_item_id: Option<Uuid>,
}

impl TransactionViewModel {
    pub fn from_transaction(tx: &BankTransaction) -> Self {
        Self {
            tx_id: tx.id,
            amount: tx.amount,
            description: tx.description.clone(),
            date: tx.date,
            actual_item_id: tx.actual_item_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetViewModel {
    pub id: Uuid,
    pub name: String,
    pub month_begins_on: MonthBeginsOn,
    pub period_id: PeriodId,
    pub overviews: Vec<BudgetingTypeOverview>,
    pub items: Vec<BudgetItemViewModel>,
    pub to_connect: Vec<TransactionViewModel>,
    pub ignored_transactions: Vec<TransactionViewModel>,
    pub currency: Currency,
}

impl BudgetViewModel {
    pub fn from_budget(budget: &Budget, period_id: PeriodId) -> Self {
        let actual_items = budget.with_period(period_id).all_actual_items();
        let budget_items = budget.list_all_items_inner();
        let transactions = budget.list_transactions_for_connection(period_id);
        let ignored_transactions = budget.list_ignored_transactions(period_id);
        
        let items = budget_items.iter().map(|bi| BudgetItemViewModel::from_item(&bi, actual_items.iter().find(|ai| ai.budget_item_id == bi.id), budget.currency)).collect::<Vec<_>>();
        let to_connect = transactions.iter().map(TransactionViewModel::from_transaction).collect::<Vec<_>>();
        let ignored_transactions = ignored_transactions.iter().map(TransactionViewModel::from_transaction).collect::<Vec<_>>();
        let mut overviews = vec!(budget.get_budgeting_overview(BudgetingType::Income, period_id),budget.get_budgeting_overview(BudgetingType::Expense, period_id), budget.get_budgeting_overview(BudgetingType::Savings, period_id));
        overviews.sort_by_key(|ov| ov.budgeting_type);
        Self {
            id: budget.id,
            name: budget.name.clone(),
            month_begins_on: budget.month_begins_on(),
            period_id,
            overviews,
            items,
            to_connect,
            ignored_transactions,
            currency: budget.currency,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Default, Hash, PartialEq, Eq)]
pub struct BudgetingTypeOverview {
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub is_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rule {
    Sum(Vec<BudgetingType>),
    Difference(BudgetingType, Vec<BudgetingType>),
    SelfDiff(BudgetingType)
}

pub enum ValueKind {
    Budgeted,
    Spent,
}

impl ValueKind {
    fn pick(&self, item: &ActualItem) -> Money {
        match self {
            ValueKind::Budgeted => item.budgeted_amount,
            ValueKind::Spent => item.actual_amount,
        }
    }
}

impl Rule {
    pub fn evaluate(
        &self,
        store: &Vec<&ActualItem>,
        kind: Option<ValueKind>,
    ) -> Money {
        match self {
            Rule::Sum(types) => types
                .iter()
                .map(|t| {
                    Self::get_sum(store, kind.as_ref().unwrap(), t)
                })
                .sum(),
            Rule::Difference(base, subtracts) => {
                let base_sum = Self::get_sum(store, kind.as_ref().unwrap(), base);
                let subtract_sum: Money = subtracts
                    .iter()
                    .map(|t| Self::get_sum(store, kind.as_ref().unwrap(), t))
                    .sum();
                base_sum - subtract_sum
            }
            Rule::SelfDiff(base ) => {
                let budget_sum = Self::get_sum(store, &ValueKind::Budgeted, base);
                let spent_sum = Self::get_sum(store, &ValueKind::Spent, base);
                budget_sum - spent_sum
            }
        }
    }

    pub fn get_sum(store: &Vec<&ActualItem>, kind: &ValueKind, base: &BudgetingType) -> Money {
        store.iter().filter(|i| i.budgeting_type() == *base).map(|i| kind.pick(i)).sum::<Money>()
    }
}

#[cfg(test)]
#[test]
fn test_calculate_rules() {
    use std::sync::{Arc, Mutex};
    use crate::models::BudgetingType::*;
    use Rule::*;
    let period_id = PeriodId::new(2025, 12);
    let mut budget_items = Vec::new();
    budget_items.push(Arc::new(Mutex::new(BudgetItem::new(Uuid::new_v4(), "Lön", Income))));
    budget_items.push(Arc::new(Mutex::new(BudgetItem::new(Uuid::new_v4(), "Hyra", Expense))));
    budget_items.push(Arc::new(Mutex::new(BudgetItem::new(Uuid::new_v4(), "Spara", Savings))));
    
    let mut store = Vec::new();
    store.push(ActualItem::new(Uuid::new_v4(), budget_items[0].clone(), period_id, Money::new_dollars(5000, Currency::SEK), Money::new_dollars(4000, Currency::SEK), None, vec![]));
    store.push(ActualItem::new(Uuid::new_v4(), budget_items[1].clone(), period_id, Money::new_dollars(3000, Currency::SEK), Money::new_dollars(2000, Currency::SEK), None, vec![]));
    store.push(ActualItem::new(Uuid::new_v4(), budget_items[2].clone(), period_id, Money::new_dollars(1000, Currency::SEK), Money::new_dollars(500, Currency::SEK), None, vec![]));
    
    
    let income_rule = Sum(vec![Income]);
    let remaining_rule = Difference(Income, vec![Expense, Savings]);

    assert_eq!(income_rule.evaluate(&store.iter().collect::<Vec<_>>(), Some(ValueKind::Budgeted)), Money::new_dollars(5000, Currency::SEK));
    assert_eq!(remaining_rule.evaluate(&store.iter().collect::<Vec<_>>(), Some(ValueKind::Budgeted)), Money::new_dollars(1000, Currency::SEK));
}