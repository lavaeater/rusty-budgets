use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, TransferRule};
use core::fmt::Display;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Records a learned internal-transfer pattern — see [`TransferRule`].
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransferRuleAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub rule_id: Uuid,
    pub outgoing_account: String,
    pub incoming_account: String,
    pub outgoing_key: Vec<String>,
    pub incoming_key: Vec<String>,
    pub tag_id: Option<Uuid>,
}

impl Display for TransferRuleAdded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TransferRuleAdded {{ budget_id: {}, outgoing_account: {}, incoming_account: {} }}",
            self.budget_id, self.outgoing_account, self.incoming_account
        )
    }
}

impl TransferRuleAddedHandler for Budget {
    fn apply_add_transfer_rule(&mut self, event: &TransferRuleAdded) -> Uuid {
        self.transfer_rules.insert(TransferRule {
            id: event.rule_id,
            outgoing_account: event.outgoing_account.clone(),
            incoming_account: event.incoming_account.clone(),
            outgoing_key: event.outgoing_key.clone(),
            incoming_key: event.incoming_key.clone(),
            tag_id: event.tag_id,
        });
        event.rule_id
    }

    fn add_transfer_rule_impl(
        &self,
        outgoing_account: String,
        incoming_account: String,
        outgoing_key: Vec<String>,
        incoming_key: Vec<String>,
        tag_id: Option<Uuid>,
    ) -> Result<TransferRuleAdded, CommandError> {
        let candidate = TransferRule {
            id: Uuid::new_v4(),
            outgoing_account: outgoing_account.clone(),
            incoming_account: incoming_account.clone(),
            outgoing_key: outgoing_key.clone(),
            incoming_key: incoming_key.clone(),
            tag_id,
        };
        if self.transfer_rules.contains(&candidate) {
            return Err(CommandError::Validation(
                "Transfer rule already exists.".to_string(),
            ));
        }
        Ok(TransferRuleAdded {
            budget_id: self.id,
            rule_id: candidate.id,
            outgoing_account,
            incoming_account,
            outgoing_key,
            incoming_key,
            tag_id,
        })
    }
}
