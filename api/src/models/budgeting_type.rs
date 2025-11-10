use core::fmt;
use core::fmt::{Display, Formatter};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use uuid::Uuid;
use crate::models::{ActualItem, BudgetItem, Currency, Money, PeriodId};

#[derive(Default,Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum BudgetingType {
    #[default]
    Income,
    Expense,
    Savings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Default, Hash, PartialEq, Eq)]
pub struct BudgetingTypeOverview {
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub is_ok: bool,
}


impl Display for BudgetingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BudgetingType::Income => "Inkomst",
                BudgetingType::Expense => "Utgift",
                BudgetingType::Savings => "Sparande",
            }
        )
    }
}

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
    use BudgetingType::*;
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
