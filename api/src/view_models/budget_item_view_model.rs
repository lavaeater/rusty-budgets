use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{ActualItem, BankTransaction, BudgetItem, BudgetingType, Currency, Money, Periodicity, TransactionAllocation};
use crate::view_models::transaction_view_model::TransactionViewModel;
use crate::view_models::budget_item_status::BudgetItemStatus;
use crate::view_models::budget_item_status::BudgetItemStatus::{Balanced, NotBudgeted, OverBudget, UnderBudget};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetItemViewModel {
    pub item_id: Uuid,
    pub actual_id: Option<Uuid>,
    pub name: String,
    pub budgeting_type: BudgetingType,
    pub tags: Vec<String>,
    pub periodicity: Periodicity,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub status: BudgetItemStatus,
    pub transactions: Vec<TransactionViewModel>,
}

impl BudgetItemViewModel {
    pub fn from_item(
        budget_item: &BudgetItem,
        actual_items: &[&ActualItem],
        currency: Currency,
        transactions: &Vec<&BankTransaction>,
        allocations: &[TransactionAllocation],
    ) -> Self {
        let actual_item = actual_items
            .iter()
            .find(|ai| ai.budget_item_id == budget_item.id);
        let item_tags = &budget_item.tags;

        if let Some(actual_item) = actual_item {
            let relevant_allocs: Vec<&TransactionAllocation> = allocations
                .iter()
                .filter(|a| {
                    a.actual_id == actual_item.id
                        || (!item_tags.is_empty() && item_tags.contains(&a.tag))
                })
                .collect();

            let mut txs = transactions
                .iter()
                .filter(|tx| {
                    tx.actual_id == Some(actual_item.id)
                        || relevant_allocs.iter().any(|a| a.transaction_id == tx.id)
                })
                .map(|tx| TransactionViewModel::from_transaction(tx))
                .collect::<Vec<_>>();
            txs.sort_by_key(|tx| tx.date);

            let allocation_amount: Money = relevant_allocs.iter().map(|a| a.amount).sum();

            let actual_amount = if allocation_amount.is_zero() {
                actual_item.actual_amount
            } else {
                allocation_amount
            };

            let status = if actual_item.budgeted_amount.is_zero() {
                NotBudgeted
            } else if actual_amount > actual_item.budgeted_amount {
                OverBudget
            } else if actual_amount < actual_item.budgeted_amount {
                UnderBudget
            } else {
                Balanced
            };

            Self {
                item_id: actual_item.budget_item_id,
                actual_id: Some(actual_item.id),
                name: actual_item.item_name.clone(),
                budgeting_type: actual_item.budgeting_type,
                tags: budget_item.tags.clone(),
                periodicity: budget_item.periodicity,
                budgeted_amount: actual_item.budgeted_amount,
                actual_amount,
                remaining_budget: actual_item.budgeted_amount - actual_amount,
                status,
                transactions: txs,
            }
        } else {
            Self {
                item_id: budget_item.id,
                actual_id: None,
                name: budget_item.name.clone(),
                budgeting_type: budget_item.budgeting_type,
                tags: budget_item.tags.clone(),
                periodicity: budget_item.periodicity,
                budgeted_amount: Money::zero(currency),
                actual_amount: Money::zero(currency),
                remaining_budget: Money::zero(currency),
                status: NotBudgeted,
                transactions: Vec::new(),
            }
        }
    }
}