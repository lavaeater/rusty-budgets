use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;
use crate::models::{ActualItem, BankTransaction, BudgetItem, BudgetingType, Currency, Money, Periodicity, Tag, TransactionAllocation};
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
    pub tag_ids: Vec<Uuid>,
    pub periodicity: Periodicity,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub status: BudgetItemStatus,
    pub transactions: Vec<TransactionViewModel>,
    /// The total amount this item needs to have available (e.g. 1200 kr for annual insurance).
    pub buffer_target: Option<Money>,
    /// Computed: buffer_target ÷ periodicity_months. None if no buffer target is set.
    pub required_monthly_contribution: Option<Money>,
}

impl BudgetItemViewModel {
    pub fn from_item(
        budget_item: &BudgetItem,
        actual_items: &[&ActualItem],
        currency: Currency,
        transactions: &Vec<&BankTransaction>,
        allocations: &[TransactionAllocation],
        budget_tags: &[Tag],
        all_period_transactions: &[&BankTransaction],
    ) -> Self {
        let actual_item = actual_items
            .iter()
            .find(|ai| ai.budget_item_id == budget_item.id);
        let resolved_tags: Vec<String> = budget_item.tag_ids
            .iter()
            .filter_map(|id| budget_tags.iter().find(|t| t.id == *id).map(|t| t.name.clone()))
            .collect();
        let item_tags = &resolved_tags;

        // Determine effective budgeting type (actual_item takes precedence over budget_item)
        let effective_budgeting_type = actual_item
            .map(|ai| ai.budgeting_type)
            .unwrap_or(budget_item.budgeting_type);

        // Compute required monthly buffer contribution from tag periodicities
        let required_monthly_contribution = budget_item.buffer_target.map(|target| {
            let max_months = budget_item.tag_ids.iter()
                .filter_map(|tid| budget_tags.iter().find(|t| t.id == *tid))
                .map(|t| match t.periodicity {
                    Periodicity::Monthly => 1i64,
                    Periodicity::Quarterly => 3,
                    Periodicity::Annual => 12,
                    Periodicity::OneOff => 1,
                })
                .max()
                .unwrap_or(12);
            target.divide(max_months)
        });

        // Compute actual amount from tagged transactions (new tagging workflow)
        let tag_ids_set: HashSet<&Uuid> = budget_item.tag_ids.iter().collect();
        let mut tagged_txs: Vec<&&BankTransaction> = all_period_transactions
            .iter()
            .filter(|tx| tx.tag_id.map_or(false, |tid| tag_ids_set.contains(&tid)))
            .collect();
        tagged_txs.sort_by_key(|tx| tx.date);
        // Raw sum of transaction amounts (negative for expenses/savings — money leaving)
        let tagged_actual_raw: Money = tagged_txs.iter().map(|tx| tx.amount).sum();
        // Normalize: budgeted amounts are always positive, so abs-normalise actual for
        // Expense and Savings so that comparisons and remaining_budget are correct.
        let tagged_actual = match effective_budgeting_type {
            BudgetingType::Expense | BudgetingType::Savings => tagged_actual_raw.abs(),
            _ => tagged_actual_raw,
        };

        if let Some(actual_item) = actual_item {
            let relevant_allocs: Vec<&TransactionAllocation> = allocations
                .iter()
                .filter(|a| {
                    a.actual_id == actual_item.id
                        || (!item_tags.is_empty() && item_tags.contains(&a.tag))
                })
                .collect();

            let allocation_amount: Money = relevant_allocs.iter().map(|a| a.amount).sum();

            // Prefer tag-based actual; fall back to allocation or stored actual
            let actual_amount = if !tagged_actual.is_zero() {
                tagged_actual
            } else if !allocation_amount.is_zero() {
                allocation_amount
            } else {
                actual_item.actual_amount
            };

            // Transactions: prefer tag-based; fall back to connected/allocation-based
            let txs = if !tagged_txs.is_empty() {
                tagged_txs.iter()
                    .map(|tx| TransactionViewModel::from_transaction(tx))
                    .collect()
            } else {
                let mut old_txs = transactions
                    .iter()
                    .filter(|tx| {
                        tx.actual_id == Some(actual_item.id)
                            || relevant_allocs.iter().any(|a| a.transaction_id == tx.id)
                    })
                    .map(|tx| TransactionViewModel::from_transaction(tx))
                    .collect::<Vec<_>>();
                old_txs.sort_by_key(|tx| tx.date);
                old_txs
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
                tags: item_tags.clone(),
                tag_ids: budget_item.tag_ids.clone(),
                periodicity: budget_item.periodicity,
                budgeted_amount: actual_item.budgeted_amount,
                actual_amount,
                remaining_budget: actual_item.budgeted_amount - actual_amount,
                status,
                transactions: txs,
                buffer_target: budget_item.buffer_target,
                required_monthly_contribution,
            }
        } else {
            let txs = tagged_txs.iter()
                .map(|tx| TransactionViewModel::from_transaction(tx))
                .collect();
            Self {
                item_id: budget_item.id,
                actual_id: None,
                name: budget_item.name.clone(),
                budgeting_type: budget_item.budgeting_type,
                tags: item_tags.clone(),
                tag_ids: budget_item.tag_ids.clone(),
                periodicity: budget_item.periodicity,
                budgeted_amount: Money::zero(currency),
                actual_amount: tagged_actual,
                remaining_budget: Money::zero(currency),
                status: NotBudgeted,
                transactions: txs,
                buffer_target: budget_item.buffer_target,
                required_monthly_contribution,
            }
        }
    }
}