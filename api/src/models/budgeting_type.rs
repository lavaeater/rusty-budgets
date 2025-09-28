use core::fmt;
use core::fmt::{Display, Formatter};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
use uuid::Uuid;
use crate::models::{BudgetItem, BudgetItemStore, Currency, Money};

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
    pub remaining_budget: Money
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
    fn pick(&self, item: &BudgetItem) -> Money {
        match self {
            ValueKind::Budgeted => item.budgeted_amount,
            ValueKind::Spent => item.actual_amount,
        }
    }
}

impl Rule {
    pub fn evaluate(
        &self,
        store: &HashMap<BudgetingType, Vec<&BudgetItem>>,
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

    pub fn get_sum(store: &HashMap<BudgetingType, Vec<&BudgetItem>>, kind: &ValueKind, base: &BudgetingType) -> Money {
        store.get(base).map_or(Money::default(), |items| items.iter().map(|i| kind.pick(i)).sum::<Money>())
    }
}

#[cfg(test)]
#[test]
fn test_calculate_rules() {
    use BudgetingType::*;
    use Rule::*;
    let mut store = BudgetItemStore::default();
    store.insert(&BudgetItem::new(Uuid::new_v4(), "Lön", Money::new_dollars(5000, Currency::SEK),None, None), Income);
    store.insert(&BudgetItem::new(Uuid::new_v4(), "Lön", Money::new_dollars(3000, Currency::SEK),None, None), Expense);
    store.insert(&BudgetItem::new(Uuid::new_v4(), "Lön", Money::new_dollars(1000, Currency::SEK),None, None), Savings);

    let income_rule = Sum(vec![Income]);
    let remaining_rule = Difference(Income, vec![Expense, Savings]);

    assert_eq!(income_rule.evaluate(&store.hash_by_type(), Some(ValueKind::Budgeted)), Money::new_dollars(5000, Currency::SEK));
    assert_eq!(remaining_rule.evaluate(&store.hash_by_type(), Some(ValueKind::Budgeted)), Money::new_dollars(1000, Currency::SEK));
}
