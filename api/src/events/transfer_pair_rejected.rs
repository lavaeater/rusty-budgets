use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "reject_transfer_pair")]
pub struct TransferPairRejected {
    budget_id: Uuid,
    outgoing_tx_id: Uuid,
    incoming_tx_id: Uuid,
}

impl Display for TransferPairRejected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransferPairRejected {{ budget_id: {}, outgoing: {}, incoming: {} }}",
            self.budget_id, self.outgoing_tx_id, self.incoming_tx_id
        )
    }
}

impl TransferPairRejectedHandler for Budget {
    fn apply_reject_transfer_pair(&mut self, event: &TransferPairRejected) -> Uuid {
        self.rejected_transfer_pairs
            .insert((event.outgoing_tx_id, event.incoming_tx_id));
        event.outgoing_tx_id
    }

    fn reject_transfer_pair_impl(
        &self,
        outgoing_tx_id: Uuid,
        incoming_tx_id: Uuid,
    ) -> Result<TransferPairRejected, CommandError> {
        Ok(TransferPairRejected {
            budget_id: self.id,
            outgoing_tx_id,
            incoming_tx_id,
        })
    }
}

