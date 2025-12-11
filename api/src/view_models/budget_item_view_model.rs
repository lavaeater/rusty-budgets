use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{ActualItem, BankTransaction, BudgetItem, BudgetingType, Currency, Money};
use crate::view_models::transaction_view_model::TransactionViewModel;
use crate::view_models::budget_item_status::BudgetItemStatus;
use crate::view_models::budget_item_status::BudgetItemStatus::{Balanced, NotBudgeted, OverBudget, UnderBudget};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetItemViewModel {
    pub item_id: Uuid,
    pub actual_id: Option<Uuid>,
    pub name: String,
    pub budgeting_type: BudgetingType,
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
    ) -> Self {
        let actual_item = actual_items
            .iter()
            .find(|ai| ai.budget_item_id == budget_item.id);
        if let Some(actual_item) = actual_item {
            let mut transactions = transactions
                .iter()
                .filter(|tx| tx.actual_id == Some(actual_item.id))
                .map(|tx| TransactionViewModel::from_transaction(tx))
                .collect::<Vec<_>>();
            transactions.sort_by_key(|tx| tx.date);

            let status = if actual_item.budgeted_amount.is_zero() {
                NotBudgeted
            } else if actual_item.actual_amount > actual_item.budgeted_amount {
                OverBudget
            } else if actual_item.actual_amount < actual_item.budgeted_amount {
                UnderBudget
            } else {
                Balanced
            };

            Self {
                item_id: actual_item.budget_item_id,
                actual_id: Some(actual_item.id),
                name: actual_item.item_name.clone(),
                budgeting_type: actual_item.budgeting_type,
                budgeted_amount: actual_item.budgeted_amount,
                actual_amount: actual_item.actual_amount,
                remaining_budget: actual_item.budgeted_amount - actual_item.actual_amount,
                status,
                transactions,
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
                status: NotBudgeted,
                transactions: Vec::new(),
            }
        }
    }
}