use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
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
        println!("Applying transaction connected event: {}", event);

        // First, extract all the data we need from the transaction (immutable borrow)
        let tx = self.get_transaction(&event.tx_id).unwrap();
        let tx_amount = tx.amount;
        let previous_item_id = tx.budget_item_id;
        let previous_item_type = match previous_item_id {
            Some(id) => self.type_for_item(&id),    
            None => None,
        };
        // End of immutable borrow - tx goes out of scope here

        // Handle previous connection if it exists
        if let Some(previous_budget_item_id) = previous_item_id {
            println!("Transaction {} is already connected to item {}", event.tx_id, previous_budget_item_id);
            println!("Previous budget item id: {}", previous_budget_item_id);
            
            let previous_budgeting_type = previous_item_type.unwrap();
            println!("Previous type id: {}", previous_budgeting_type);

            // Update budget total (remove from previous item)
            self.update_budget_actual_amount(&previous_budgeting_type, &-tx_amount);
            self.add_actual_amount_to_item(&previous_budget_item_id, &-tx_amount);
        }

        // Now we can mutably borrow to update the transaction
        let tx_mut = self.get_transaction_mut(&event.tx_id).unwrap();
        tx_mut.budget_item_id = Some(event.item_id);
        // End of mutable borrow

        // Update the new item
        let budgeting_type = self.type_for_item(&event.item_id).unwrap();

        // Update budget total (add to new item)
        self.update_budget_actual_amount(&budgeting_type, &tx_amount);
        self.add_actual_amount_to_item(&event.item_id, &tx_amount);
        self.recalc_overview();

        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        item_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if self.contains_transaction(&tx_id) && self.contains_budget_item(&item_id) {
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
