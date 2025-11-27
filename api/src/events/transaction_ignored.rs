use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, PeriodId, BudgetingType};
use core::fmt::Display;
use dioxus::logger::tracing;
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
        let tx = self.get_transaction(event.tx_id).unwrap();
        let tx_amount = tx.amount;
        let previous_actual_id = tx.actual_id;
        let period_id = PeriodId::from_date(tx.date, self.month_begins_on());
        
        if let Some(previous_actual_id) = previous_actual_id {
            let previous_actual = self.with_period(period_id).get_actual(previous_actual_id).unwrap();
            let bt = previous_actual.budgeting_type;
            self.with_period_mut(period_id).mutate_actual(previous_actual_id, |a| {
                let adjusted_amount = if cost_types.contains(&bt) {
                    -tx_amount
                } else {
                    tx_amount
                };
                a.actual_amount -= adjusted_amount;
            });
        }

        self.set_transaction_ignored(event.tx_id, period_id);
        event.tx_id
    }

    fn ignore_transaction_impl(
        &self,
        tx_id: Uuid
    ) -> Result<TransactionIgnored, CommandError> {
        if self.contains_transaction(tx_id) {
            Ok(TransactionIgnored {
                budget_id: self.id,
                tx_id,
            })
        } else {
            let bork = &self.all_transactions();
            tracing::error!("These are the transactions: {:?}", bork);
            Err(CommandError::Validation(
                format!("Transaction {} does not exist", tx_id),
            ))
        }
    }
}
