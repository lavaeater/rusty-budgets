use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{Budget, BudgetingType, Currency, MonthBeginsOn, PeriodId, Tag};
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
    pub currency: Currency,
    pub tags: Vec<Tag>,
}

impl BudgetViewModel {
    pub fn from_budget(budget: &Budget, period_id: PeriodId) -> Self {
        let actual_items = budget.all_actuals(period_id);
        let budget_items = budget.all_items();
        let transactions = budget.unconnected_transactions(period_id);
        let ignored_transactions = budget.ignored_transactions(period_id);
        let connected_transactions = budget.connected_transactions(period_id);

        let period_allocations = budget
            .get_period(period_id)
            .map(|p| p.allocations.clone())
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
        let potential_transfers = budget
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
            .collect::<Vec<_>>();

        let mut overviews = vec![
            budget.get_budgeting_overview(BudgetingType::Income, period_id),
            budget.get_budgeting_overview(BudgetingType::Expense, period_id),
            budget.get_budgeting_overview(BudgetingType::Savings, period_id),
            budget.get_budgeting_overview(BudgetingType::InternalTransfer, period_id),
        ];
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
            potential_transfers,
            currency: budget.currency,
            tags: budget.tags.clone(),
        }
    }
}