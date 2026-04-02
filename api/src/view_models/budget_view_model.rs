use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{Budget, BudgetingType, Currency, MatchRule, MonthBeginsOn, PeriodId, Tag};
use crate::view_models::allocation_view_model::AllocationViewModel;
use crate::view_models::budget_item_view_model::BudgetItemViewModel;
use crate::view_models::budgeting_type_overview::BudgetingTypeOverview;
use crate::view_models::transaction_view_model::TransactionViewModel;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct TransferPair {
    pub outgoing: TransactionViewModel,
    pub incoming: TransactionViewModel,
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
    pub potential_transfers: Vec<TransferPair>,
    pub potential_transfer_count: usize,
    pub currency: Currency,
    pub tags: Vec<Tag>,
    pub match_rules: Vec<MatchRule>,
    pub untagged_transaction_count: usize,
}

impl BudgetViewModel {
    pub fn from_budget(budget: &Budget, period_id: PeriodId) -> Self {
        let t = std::time::Instant::now();
        let actual_items = budget.all_actuals(period_id);
        let budget_items = budget.all_items();
        let transactions = budget.unconnected_transactions(period_id);
        let ignored_transactions = budget.ignored_transactions(period_id);
        let connected_transactions = budget.connected_transactions(period_id);

        let period_allocations = budget
            .get_period(period_id)
            .map(|p| p.allocations.clone())
            .unwrap_or_default();

        let all_period_transactions: Vec<&crate::models::BankTransaction> = budget
            .get_period(period_id)
            .map(|p| p.transactions.iter().filter(|t| !t.ignored).collect())
            .unwrap_or_default();

        let items = budget_items
            .iter()
            .map(|bi| {
                BudgetItemViewModel::from_item(
                    bi,
                    &actual_items,
                    budget.currency,
                    &connected_transactions,
                    &period_allocations,
                    &budget.tags,
                    &all_period_transactions,
                )
            })
            .collect::<Vec<_>>();
        let to_connect = transactions
            .iter()
            .map(|tx| {
                let allocs = budget
                    .allocations_for_transaction(tx.id)
                    .iter()
                    .map(|a| AllocationViewModel::from_allocation(a))
                    .collect::<Vec<_>>();
                TransactionViewModel::from_transaction_with_allocations(tx, allocs)
            })
            .collect::<Vec<_>>();
        let ignored_transactions = ignored_transactions
            .iter()
            .map(|tx| TransactionViewModel::from_transaction(tx))
            .collect::<Vec<_>>();
        let t_transfers = std::time::Instant::now();
        let all_transfer_pairs: Vec<TransferPair> = budget
            .potential_internal_transfers()
            .into_iter()
            .filter_map(|(out_id, in_id)| {
                let out_tx = budget.get_transaction(out_id)?;
                let in_tx = budget.get_transaction(in_id)?;
                Some(TransferPair {
                    outgoing: TransactionViewModel::from_transaction(out_tx),
                    incoming: TransactionViewModel::from_transaction(in_tx),
                })
            })
            .collect();
        let transfer_ids: std::collections::HashSet<uuid::Uuid> = all_transfer_pairs
            .iter()
            .flat_map(|p| [p.outgoing.tx_id, p.incoming.tx_id])
            .collect();
        tracing::info!("[perf] from_budget/potential_transfers ({} pairs): {:?}", all_transfer_pairs.len(), t_transfers.elapsed());
        let potential_transfer_count = all_transfer_pairs.len();
        let potential_transfers = all_transfer_pairs.into_iter().take(10).collect::<Vec<_>>();

        // Compute overviews from BudgetItemViewModels (which have tag-based actual amounts)
        // rather than from the RulePackages/ActualItem system which is never updated by
        // the tagging workflow.
        let sum_budgeted = |bt: BudgetingType| -> crate::models::Money {
            items.iter().filter(|i| i.budgeting_type == bt).map(|i| i.budgeted_amount).sum()
        };
        let sum_actual = |bt: BudgetingType| -> crate::models::Money {
            items.iter().filter(|i| i.budgeting_type == bt).map(|i| i.actual_amount).sum()
        };
        let income_budgeted = sum_budgeted(BudgetingType::Income);
        let expense_budgeted = sum_budgeted(BudgetingType::Expense);
        let savings_budgeted = sum_budgeted(BudgetingType::Savings);
        let transfer_budgeted = sum_budgeted(BudgetingType::InternalTransfer);
        let income_actual = sum_actual(BudgetingType::Income);
        let expense_actual = sum_actual(BudgetingType::Expense);
        let savings_actual = sum_actual(BudgetingType::Savings);
        let transfer_actual = sum_actual(BudgetingType::InternalTransfer);
        // Income remaining = income budgeted minus what is allocated to expenses + savings
        let income_remaining = income_budgeted - expense_budgeted - savings_budgeted;
        let mut overviews = vec![
            BudgetingTypeOverview {
                budgeting_type: BudgetingType::Income,
                budgeted_amount: income_budgeted,
                actual_amount: income_actual,
                remaining_budget: income_remaining,
                is_ok: income_remaining >= crate::models::Money::zero(budget.currency),
            },
            BudgetingTypeOverview {
                budgeting_type: BudgetingType::Expense,
                budgeted_amount: expense_budgeted,
                actual_amount: expense_actual,
                remaining_budget: expense_budgeted - expense_actual,
                is_ok: expense_actual <= expense_budgeted,
            },
            BudgetingTypeOverview {
                budgeting_type: BudgetingType::Savings,
                budgeted_amount: savings_budgeted,
                actual_amount: savings_actual,
                remaining_budget: savings_budgeted - savings_actual,
                is_ok: savings_actual <= savings_budgeted,
            },
            BudgetingTypeOverview {
                budgeting_type: BudgetingType::InternalTransfer,
                budgeted_amount: transfer_budgeted,
                actual_amount: transfer_actual,
                remaining_budget: transfer_budgeted - transfer_actual,
                is_ok: true,
            },
        ];
        overviews.sort_by_key(|ov| ov.budgeting_type);
        let untagged_transaction_count = budget
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|tx| tx.tag_id.is_none() && !tx.ignored && !transfer_ids.contains(&tx.id))
            .count();
        let match_rules = budget.match_rules.iter().cloned().collect::<Vec<_>>();
        tracing::info!("[perf] from_budget/total: {:?}", t.elapsed());

        Self {
            id: budget.id,
            name: budget.name.clone(),
            month_begins_on: budget.month_begins_on(),
            period_id,
            overviews,
            items,
            to_connect,
            ignored_transactions,
            potential_transfers,
            potential_transfer_count,
            currency: budget.currency,
            tags: budget.tags.clone(),
            match_rules,
            untagged_transaction_count,
        }
    }
}