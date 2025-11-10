use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetingType, PeriodId};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionConnected {
    budget_id: Uuid,
    tx_id: Uuid,
    actual_id: Uuid,
}

impl Display for TransactionConnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransactionConnected {{ budget_id: {}, tx_id: {}, actual_id: {} }}",
            self.budget_id, self.tx_id, self.actual_id
        )
    }
}

impl TransactionConnectedHandler for Budget {
    fn apply_connect_transaction(&mut self, event: &TransactionConnected) -> Uuid {
        let cost_types = Vec::from([BudgetingType::Expense, BudgetingType::Savings]);

        let tx = self.get_transaction(event.tx_id).unwrap();
        let period_id = PeriodId::from_date(tx.date, *self.month_begins_on());
        let tx_amount = tx.amount;
        if let Some(previous_id) = tx.actual_item_id {
            let bt = self.with_period(period_id).get_actual(previous_id).unwrap().budgeting_type();
            self.with_period_mut(period_id)
                .mutate_actual(previous_id, |a| {
                    let adjusted_amount = if cost_types.contains(&bt) {
                        -tx_amount
                    } else {
                        tx_amount
                    };
                    a.actual_amount -= adjusted_amount;
                });
        }
        let bt = self.with_period(period_id).get_actual(event.actual_id).unwrap().budgeting_type();
        let tx_mut = self.get_transaction_mut(event.tx_id).unwrap();
        tx_mut.actual_item_id = Some(event.actual_id);

        self.with_period_mut(period_id)
            .mutate_actual(event.actual_id, |a| {
                let adjusted_amount = if cost_types.contains(&bt) {
                    -tx_amount
                } else {
                    tx_amount
                };
                a.actual_amount += adjusted_amount;
            });

        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if let Some(period) = self.get_period_for_transaction(tx_id) {
            if period.contains_actual_for_item(actual_id) {
                Ok(TransactionConnected {
                    budget_id: self.id,
                    tx_id,
                    actual_id,
                })
            } else {
                Err(CommandError::Validation(
                    "Actual does not exist for period.".to_string(),
                ))
            }
        } else {
            Err(CommandError::Validation(
                "Transaction does not exist.".to_string(),
            ))
        }
    }
}
