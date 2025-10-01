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
        // Connect transaction to item
        let tx = self.bank_transactions.get_mut(&event.tx_id).unwrap();

        if tx.budget_item_id.is_some() {
            println!("Transaction: {}", tx);
            println!(
                "Transaction {} is already connected to item {}",
                event.tx_id,
                tx.budget_item_id.unwrap()
            );
            let previous_budget_item_id = tx.budget_item_id.unwrap();
            println!("Previous budget item id: {}", previous_budget_item_id);
            let previous_budgeting_type = self
                .budget_items
                .type_for(&previous_budget_item_id)
                .unwrap();
            println!("Previous type id: {}", previous_budgeting_type);

            //Update budget total
            self.actual_by_type
                .entry(*previous_budgeting_type)
                .and_modify(|v| {
                    *v -= tx.amount;
                });

            self.budget_items.add_actual_amount(&previous_budget_item_id, -tx.amount);
        }
        tx.budget_item_id = Some(event.item_id);
        // Update group
        let budgeting_type = self.budget_items
            .type_for(&event.item_id).unwrap();

        //Update budget total
        self.actual_by_type
            .entry(*budgeting_type)
            .and_modify(|v| {
                *v += tx.amount;
            })
            .or_insert(tx.amount);
        // Update item
        self.budget_items.add_actual_amount(&event.item_id, tx.amount);
        self.recalc_overview();
        event.tx_id
    }

    fn connect_transaction_impl(
        &self,
        tx_id: Uuid,
        item_id: Uuid,
    ) -> Result<TransactionConnected, CommandError> {
        if self.bank_transactions.contains(&tx_id) && self.budget_items.contains(&item_id) {
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
