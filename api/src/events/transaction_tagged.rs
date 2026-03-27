use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionTagged {
    budget_id: Uuid,
    tx_id: Uuid,
    tag_id: Uuid,
}

impl TransactionTaggedHandler for Budget {
    fn apply_tag_transaction(&mut self, event: &TransactionTagged) -> Uuid {
        if let Some(tx) = self.get_transaction_mut(event.tx_id) {
            tx.tag_id = Some(event.tag_id);
        }
        event.tx_id
    }

    fn tag_transaction_impl(
        &self,
        tx_id: Uuid,
        tag_id: Uuid,
    ) -> Result<TransactionTagged, CommandError> {
        if !self.contains_transaction(tx_id) {
            return Err(CommandError::Validation(
                format!("Transaction {} does not exist", tx_id),
            ));
        }
        if !self.contains_tag(tag_id) {
            return Err(CommandError::Validation(
                format!("Tag {} does not exist", tag_id),
            ));
        }
        Ok(TransactionTagged {
            budget_id: self.id,
            tx_id,
            tag_id,
        })
    }
}
