use chrono::{DateTime, Utc};
use crate::models::{ActualItem, BankTransaction, Budget, BudgetItem, BudgetingType, Currency, Money, MonthBeginsOn, PeriodId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetItemViewModel {
    item_id: Uuid,
    actual_id: Option<Uuid>,
    name: String,
    budgeting_type: BudgetingType,
    budgeted_amount: Money,
    actual_amount: Money,
    remaining_budget: Money,
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
    id: Uuid,
    name: String,
    month_begins_on: MonthBeginsOn,
    period_id: PeriodId,
    items: Vec<BudgetItemViewModel>,
    to_connect: Vec<TransactionViewModel>,
    ignored_transactions: Vec<TransactionViewModel>,
    currency: Currency,
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
        
        Self {
            id: budget.id,
            name: budget.name.clone(),
            month_begins_on: budget.month_begins_on(),
            period_id,
            items,
            to_connect,
            ignored_transactions,
            currency: budget.currency,
        }
    }
}
