use crate::models::{
    ActualItem, BankTransaction, BudgetItem, BudgetingType, Currency, Money, Periodicity, Tag,
    TransactionAllocation,
};
use crate::view_models::budget_item_status::BudgetItemStatus;
use crate::view_models::budget_item_status::BudgetItemStatus::{
    Balanced, NotBudgeted, OverBudget, UnderBudget,
};
use crate::view_models::transaction_view_model::TransactionViewModel;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

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
    /// Balance carried in from previous periods. Zero when carryover is off.
    /// Negative means the category is still covering an earlier overspend.
    pub carried_over: Money,
    /// The envelope figure: `carried_over + budgeted - actual`. This is what is
    /// really left to spend, as opposed to `remaining_budget`, which looks at
    /// this period alone.
    pub available: Money,
    pub status: BudgetItemStatus,
    pub transactions: Vec<TransactionViewModel>,
    /// The total amount this item needs to have available (e.g. 1200 kr for annual insurance).
    pub buffer_target: Option<Money>,
    /// Computed: `buffer_target` ÷ `periodicity_months`. None if no buffer target is set.
    pub required_monthly_contribution: Option<Money>,
}

impl BudgetItemViewModel {
    #[allow(clippy::too_many_lines, clippy::too_many_arguments)]
    pub fn from_item(
        budget_item: &BudgetItem,
        actual_items: &[&ActualItem],
        currency: Currency,
        transactions: &Vec<&BankTransaction>,
        allocations: &[TransactionAllocation],
        budget_tags: &[Tag],
        all_period_transactions: &[&BankTransaction],
        carried_over: Money,
    ) -> Self {
        let actual_item = actual_items
            .iter()
            .find(|ai| ai.budget_item_id == budget_item.id);
        let resolved_tags: Vec<String> = budget_item
            .tag_ids
            .iter()
            .filter_map(|id| {
                budget_tags
                    .iter()
                    .find(|t| t.id == *id)
                    .map(|t| t.name.clone())
            })
            .collect();
        let item_tags = &resolved_tags;

        // Determine effective budgeting type (actual_item takes precedence over budget_item)
        let effective_budgeting_type = actual_item
            .map_or(budget_item.budgeting_type, |ai| ai.budgeting_type);

        // Spread the buffer target over the longest billing cycle among the
        // item's tags. `Variable` tags contribute no cycle — they are budgeted
        // as they are spent, never buffered.
        let required_monthly_contribution = budget_item.buffer_target.map(|target| {
            let max_months = budget_item
                .tag_ids
                .iter()
                .filter_map(|tid| budget_tags.iter().find(|t| t.id == *tid))
                .filter_map(|t| t.cost_kind.cycle_months())
                .max()
                .unwrap_or(12);
            target.divide(max_months)
        });

        // Compute actual amount from tagged transactions (new tagging workflow)
        let tag_ids_set: HashSet<&Uuid> = budget_item.tag_ids.iter().collect();
        let mut tagged_txs: Vec<&&BankTransaction> = all_period_transactions
            .iter()
            .filter(|tx| tx.tag_id.is_some_and(|tid| tag_ids_set.contains(&tid)))
            .collect();
        tagged_txs.sort_by_key(|tx| tx.date);
        // Raw sum of transaction amounts (negative for expenses/savings — money leaving)
        let tagged_actual_raw: Money = tagged_txs.iter().map(|tx| tx.amount).sum();
        // Normalize: budgeted amounts are always positive, so sign-flip actual for
        // Expense and Savings so a net spend compares as positive. A net *refund*
        // (e.g. a return posted the month after the purchase, so this period has no
        // offsetting expense to net against) becomes a negative actual — a credit
        // that increases `remaining_budget`/`available` — rather than being
        // abs()-ed into looking like additional spending.
        let tagged_actual = match effective_budgeting_type {
            BudgetingType::Expense | BudgetingType::Savings => -tagged_actual_raw,
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
            let txs = if tagged_txs.is_empty() {
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
            } else {
                tagged_txs
                    .iter()
                    .map(|tx| TransactionViewModel::from_transaction(tx))
                    .collect()
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
                carried_over,
                available: carried_over + actual_item.budgeted_amount - actual_amount,
                status,
                transactions: txs,
                buffer_target: budget_item.buffer_target,
                required_monthly_contribution,
            }
        } else {
            let txs = tagged_txs
                .iter()
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
                carried_over,
                available: carried_over - tagged_actual,
                status: NotBudgeted,
                transactions: txs,
                buffer_target: budget_item.buffer_target,
                required_monthly_contribution,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::PeriodId;
    use chrono::Utc;

    fn tagged_tx(amount_cents: i64, tag_id: Uuid) -> BankTransaction {
        BankTransaction {
            id: Uuid::new_v4(),
            account_number: "12345678901".to_string(),
            amount: Money::new_cents(amount_cents, Currency::SEK),
            description: "test".to_string(),
            date: Utc::now(),
            actual_id: None,
            balance: Money::zero(Currency::SEK),
            ignored: false,
            tag_id: Some(tag_id),
        }
    }

    /// A return posted the month *after* the purchase lands in a period with
    /// no offsetting expense for that tag, so the period's net is a positive
    /// refund. That must show up as a credit (negative actual, boosting
    /// `remaining_budget`/`available`), not as additional spending.
    #[test]
    fn refund_only_period_reports_negative_actual_as_a_credit() {
        let tag_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();
        let mut budget_item = BudgetItem::new(item_id, "Clothes", BudgetingType::Expense);
        budget_item.tag_ids.push(tag_id);

        let actual_item = ActualItem::new(
            Uuid::new_v4(),
            "Clothes",
            item_id,
            BudgetingType::Expense,
            PeriodId::new(2026, 9),
            Money::new_cents(50_000, Currency::SEK), // 500 kr budgeted
            Money::zero(Currency::SEK),
            None,
            Vec::new(),
        );

        // Only transaction this period: a 300 kr refund for a purchase that
        // was expensed last month.
        let refund = tagged_tx(30_000, tag_id);
        let all_period_transactions: Vec<&BankTransaction> = vec![&refund];

        let vm = BudgetItemViewModel::from_item(
            &budget_item,
            &[&actual_item],
            Currency::SEK,
            &Vec::new(),
            &[],
            &[],
            &all_period_transactions,
            Money::zero(Currency::SEK),
        );

        assert_eq!(vm.actual_amount, Money::new_cents(-30_000, Currency::SEK));
        assert_eq!(vm.remaining_budget, Money::new_cents(80_000, Currency::SEK));
        assert_eq!(vm.status, UnderBudget);
    }
}
