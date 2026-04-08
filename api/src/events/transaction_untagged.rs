use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionUntagged {
    budget_id: Uuid,
    tx_id: Uuid,
}

impl TransactionUntaggedHandler for Budget {
    fn apply_do_transaction_untagged(&mut self, event: &TransactionUntagged) -> Uuid {
        if let Some(tx) = self.get_transaction_mut(event.tx_id) {
            tx.tag_id = None;
        }
        event.tx_id
    }

    fn do_transaction_untagged_impl(
        &self,
        tx_id: Uuid,
    ) -> Result<TransactionUntagged, CommandError> {
        if !self.contains_transaction(tx_id) {
            return Err(CommandError::Validation(
                format!("Transaction {} does not exist", tx_id),
            ));
        }
        Ok(TransactionUntagged {
            budget_id: self.id,
            tx_id,
        })
    }
}
