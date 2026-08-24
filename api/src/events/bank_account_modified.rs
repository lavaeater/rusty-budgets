use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{BankAccountType, Budget};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BankAccountModified {
    pub budget_id: Uuid,
    pub account_id: Uuid,
    pub account_type: BankAccountType,
}

impl BankAccountModifiedHandler for Budget {
    fn apply_modify_bank_account(&mut self, event: &BankAccountModified) -> Uuid {
        if let Some(account) = self.accounts.iter_mut().find(|a| a.id == event.account_id) {
            account.account_type = event.account_type;
        }
        event.account_id
    }

    fn modify_bank_account_impl(
        &self,
        account_id: Uuid,
        account_type: BankAccountType,
    ) -> Result<BankAccountModified, CommandError> {
        if !self.accounts.iter().any(|a| a.id == account_id) {
            return Err(CommandError::NotFound("Account not found".to_string()));
        }
        Ok(BankAccountModified {
            budget_id: self.id,
            account_id,
            account_type,
        })
    }
}
