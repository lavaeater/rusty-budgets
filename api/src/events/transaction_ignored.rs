use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetPeriodId, BudgetingType};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TransactionIgnored,
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionIgnored {
    budget_id: Uuid,
    tx_id: Uuid
}

impl Display for TransactionIgnored {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransactionIgnored {{ budget_id: {}, tx_id: {} }}",
            self.budget_id, self.tx_id
        )
    }
}

impl TransactionIgnoredHandler for Budget {
    fn apply_ignore_transaction(&mut self, event: &TransactionIgnored) -> Uuid {
        let cost_types = Vec::from([BudgetingType::Expense, BudgetingType::Savings]);

        // First, extract all the data we need from the transaction (immutable borrow)
        let tx = self.get_transaction(&event.tx_id).unwrap();
        let tx_amount = tx.amount;
        let previous_item_id = tx.budget_item_id;
        let previous_item_type = match previous_item_id {
            Some(id) => self.type_for_item(&id),
            None => None,
        };
        let budget_period_id = BudgetPeriodId::from_date(tx.date, *self.month_begins_on());
        // End of immutable borrow - tx goes out of scope here

        // Handle previous connection if it exists
        if let Some(previous_budget_item_id) = previous_item_id {
            let previous_budgeting_type = previous_item_type.unwrap();
            // Adjust amount for cost types (negate for Expense/Savings)
            let adjusted_amount = if cost_types.contains(&previous_budgeting_type) {
                -tx_amount
            } else {
                tx_amount
            };
            // Update budget total (remove from previous item)
            self.update_budget_actual_amount(budget_period_id, &previous_budgeting_type, &-adjusted_amount);
            self.add_actual_amount_to_item(budget_period_id, &previous_budget_item_id, &-adjusted_amount);
        }

        // Now we can mutably borrow to update the transaction
        self.with_period_mut(budget_period_id).transactions.ignore_transaction(&event.tx_id);
        // End of mutable borrow
        self.recalc_overview(Some(budget_period_id));

        event.tx_id
    }

    fn ignore_transaction_impl(
        &self,
        tx_id: Uuid
    ) -> Result<TransactionIgnored, CommandError> {
        if self.contains_transaction(&tx_id) {
            Ok(TransactionIgnored {
                budget_id: self.id,
                tx_id,
            })
        } else {
            Err(CommandError::Validation(
                "Transaction does not exist.",
            ))
        }
    }
}
