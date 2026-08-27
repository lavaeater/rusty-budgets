use crate::models::{Budget, BudgetingType, Currency, Money, PeriodId};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportTagViewModel {
    pub tag_id: Uuid,
    pub name: String,
    pub actual_amount: Money,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportItemViewModel {
    pub item_id: Uuid,
    pub name: String,
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub tags: Vec<ReportTagViewModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ReportViewModel {
    pub currency: Currency,
    pub items: Vec<ReportItemViewModel>,
    /// Every year that has at least one period with data, sorted ascending —
    /// used to populate the year select box.
    pub available_years: Vec<i32>,
}

impl ReportViewModel {
    /// Builds the report for `year`, restricted to `month` when given, or the
    /// entire year when `month` is `None`.
    pub fn from_budget(budget: &Budget, year: i32, month: Option<u32>) -> Self {
        let period_ids: Vec<PeriodId> = match month {
            Some(m) => vec![PeriodId::new(year, m)],
            None => (1..=12).map(|m| PeriodId::new(year, m)).collect(),
        };

        let items = budget
            .all_items()
            .iter()
            .map(|budget_item| {
                let budgeted_amount: Money = period_ids
                    .iter()
                    .filter_map(|pid| budget.get_period(*pid))
                    .flat_map(|p| p.actual_items.iter())
                    .filter(|ai| ai.budget_item_id == budget_item.id)
                    .map(|ai| ai.budgeted_amount)
                    .sum();

                let period_transactions: Vec<_> = period_ids
                    .iter()
                    .filter_map(|pid| budget.get_period(*pid))
                    .flat_map(|p| p.transactions.iter())
                    .filter(|tx| !tx.ignored)
                    .collect();

                let normalize = |raw: Money| -> Money {
                    match budget_item.budgeting_type {
                        BudgetingType::Expense | BudgetingType::Savings => raw.abs(),
                        _ => raw,
                    }
                };

                let tags: Vec<ReportTagViewModel> = budget_item
                    .tag_ids
                    .iter()
                    .filter_map(|tag_id| {
                        let tag = budget.tags.iter().find(|t| t.id == *tag_id)?;
                        let raw: Money = period_transactions
                            .iter()
                            .filter(|tx| tx.tag_id == Some(*tag_id))
                            .map(|tx| tx.amount)
                            .sum();
                        Some(ReportTagViewModel {
                            tag_id: *tag_id,
                            name: tag.name.clone(),
                            actual_amount: normalize(raw),
                        })
                    })
                    .collect();

                let actual_amount: Money = tags.iter().map(|t| t.actual_amount).sum();

                ReportItemViewModel {
                    item_id: budget_item.id,
                    name: budget_item.name.clone(),
                    budgeting_type: budget_item.budgeting_type,
                    budgeted_amount,
                    actual_amount,
                    tags,
                }
            })
            .collect();

        let available_years: HashSet<i32> = budget.periods.iter().map(|p| p.id.year).collect();
        let mut available_years: Vec<i32> = available_years.into_iter().collect();
        available_years.sort_unstable();

        Self {
            currency: budget.currency,
            items,
            available_years,
        }
    }
}
