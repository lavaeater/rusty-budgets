use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, BudgetPeriodId, BudgetingType};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// TransactionConnected,
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionConnected {
    budget_id: Uuid,
    tx_id: Uuid,
    item_id: Uuid,
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
        let previous_item_id = tx.budget_item_id;
        let previous_item_type = match previous_item_id {
            Some(id) => self.type_for_item(&id),
            None => None,
        };

        let budget_period_id = BudgetPeriodId::from_date(tx.date, *self.month_begins_on());
        // End of immutable borrow - tx goes out of scope here

        //TODO: Needs to be aware of budget_periods!
        /*
        Operations that are time-sensitive should probably be scoped like this:

        self.for_period(date).update_budget_actual_amount(&previous_budgeting_type, &-adjusted_amount);
        self.for_period(date).add_actual_amount_to_item(&previous_budget_item_id, &-adjusted_amount);
        self.for_period(date).add_actual_amount_to_item(&event.item_id, &adjusted_amount);
        self.for_period(date).recalc_overview();

        This makes it more obvious what is happening.
         */
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
            self.with_period_mut(&budget_period_id).actual_by_type
                    .entry(previous_budgeting_type)
                    .and_modify(|v| {
                        *v -= adjusted_amount;
                    });
            self.with_period_mut(&budget_period_id).budget_items
                    .add_actual_amount(&previous_budget_item_id, &-adjusted_amount);
        }

        // Now we can mutably borrow to update the transaction
        let tx_mut = self.get_transaction_mut(&event.tx_id).unwrap();
        tx_mut.budget_item_id = Some(event.item_id);
        // End of mutable borrow

        // Update the new item
        let budgeting_type = &self
            .with_period(&budget_period_id)
            .budget_items.type_for(&event.item_id)
            .unwrap().clone();

        // Adjust amount for cost types (negate for Expense/Savings)
        let adjusted_amount = if cost_types.contains(&budgeting_type) {
            -tx_amount
        } else {
            tx_amount
        };

        // Update budget total (add to new item)
        self.update_budget_actual_amount(&budget_period_id, &budgeting_type, &adjusted_amount);
        self.add_actual_amount_to_item(&budget_period_id, &event.item_id, &adjusted_amount);
        self.recalc_overview(Some(&budget_period_id));

        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        item_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if self.contains_transaction(&tx_id) && self.contains_budget_item(&item_id) {
            if let Some(tx) = self.get_transaction(&tx_id) {
                if tx.budget_item_id == Some(item_id) {
                    return Err(CommandError::Validation(
                        "Transaction is already connected to this item.",
                    ));
                }
            }
            Ok(TransactionConnected {
                budget_id: self.id,
                tx_id,
                item_id,
            })
        } else {
            Err(CommandError::Validation(
                "Transaction or item does not exist.",
            ))
        }
    }
}
