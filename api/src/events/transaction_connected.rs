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
            "TransactionConnected {{ budget_id: {}, tx_id: {}, item_id: {} }}",
            self.budget_id, self.tx_id, self.item_id
        )
    }
}

impl TransactionConnectedHandler for Budget {
    fn apply_connect_transaction(&mut self, event: &TransactionConnected) -> Uuid {
        let cost_types = Vec::from([BudgetingType::Expense, BudgetingType::Savings]);

        // First, extract all the data we need from the transaction (immutable borrow)
        let tx = self.get_transaction(&event.tx_id).unwrap();
        let tx_amount = tx.amount;
        let previous_item_id = tx.actual_item_id;
        let previous_item_type = match previous_item_id {
            Some(id) => self.type_for_item(&id),
            None => None,
        };

        let budget_period_id = PeriodId::from_date(tx.date, *self.month_begins_on());
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
            self.get_period_mut(budget_period_id)
                .actual_by_type
                .entry(previous_budgeting_type)
                .and_modify(|v| {
                    *v -= adjusted_amount;
                });
            self.get_period_mut(budget_period_id)
                .budget_items
                .add_actual_amount(&previous_budget_item_id, &-adjusted_amount);
        }
        if !self
            .get_period(budget_period_id)
            .budget_items
            .contains(&event.item_id)
        {
            let item = self.budget_items.get(&event.item_id).unwrap().clone();
            let type_for = self.budget_items.type_for(&event.item_id).unwrap().clone();
            self.get_period_mut(budget_period_id)
                .budget_items
                .insert(&item, type_for);
        }

        // Now we can mutably borrow to update the transaction
        let tx_mut = self.get_transaction_mut(&event.tx_id).unwrap();

        tx_mut.actual_item_id = Some(event.item_id);
        // End of mutable borrow

        // Update the new item
        let budgeting_type = &self
            .get_period(budget_period_id)
            .budget_items
            .type_for(&event.item_id)
            .unwrap()
            .clone();

        // Adjust amount for cost types (negate for Expense/Savings)
        let adjusted_amount = if cost_types.contains(&budgeting_type) {
            -tx_amount
        } else {
            tx_amount
        };

        // Update budget total (add to new item)
        self.update_budget_actual_amount(budget_period_id, &budgeting_type, &adjusted_amount);
        self.add_actual_amount_to_item(budget_period_id, &event.item_id, &adjusted_amount);
        self.recalc_overview(Some(budget_period_id));

        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        actual_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if let Some(period) = self.get_period_for_transaction(&tx_id) {
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
