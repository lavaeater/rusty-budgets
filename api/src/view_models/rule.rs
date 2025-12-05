use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use crate::models::{ActualItem, BudgetingType, Money};
use crate::view_models::value_kind::ValueKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rule {
    Sum(Vec<BudgetingType>),
    Difference(BudgetingType, Vec<BudgetingType>),
    SelfDiff(BudgetingType),
}

impl Rule {
    pub fn evaluate(&self, store: &Vec<ActualItem>, kind: Option<ValueKind>) -> Money {
        match self {
            Rule::Sum(types) => types
                .iter()
                .map(|t| Self::get_sum(store, kind.as_ref().unwrap(), t))
                .sum(),
            Rule::Difference(base, subtracts) => {
                tracing::info!("Base: {:?}", base);
                tracing::info!("Subtracts: {:?}", subtracts);
                tracing::info!("Kind: {:?}", kind);
                let base_sum = Self::get_sum(store, kind.as_ref().unwrap(), base);
                let subtract_sum: Money = subtracts
                    .iter()
                    .map(|t| Self::get_sum(store, kind.as_ref().unwrap(), t))
                    .sum();
                base_sum - subtract_sum
            }
            Rule::SelfDiff(base) => {
                let budget_sum = Self::get_sum(store, &ValueKind::Budgeted, base);
                let spent_sum = Self::get_sum(store, &ValueKind::Spent, base);
                budget_sum - spent_sum
            }
        }
    }

    pub fn get_sum(store: &Vec<ActualItem>, kind: &ValueKind, base: &BudgetingType) -> Money {
        store
            .iter()
            .filter(|i| i.budgeting_type == *base)
            .map(|i| kind.pick(i))
            .sum::<Money>()
    }
}