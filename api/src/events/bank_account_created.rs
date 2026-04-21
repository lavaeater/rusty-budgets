use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{BankAccount, Budget, Currency, Money};
use cqrs_macros::DomainEvent;
use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct BankAccountCreated {
    pub budget_id: Uuid,
    #[event_id]
    pub account_id: Uuid,
    pub account_number: String,
    pub description: String,
}

impl BankAccountCreatedHandler for Budget {
    fn apply_create_bank_account(&mut self, event: &BankAccountCreated) -> Uuid {
        self.add_account(BankAccount {
            id: event.account_id,
            account_number: event.account_number.clone(),
            description: event.description.clone(),
            currency: String::new(),
            balance: Money::zero(Currency::SEK),
        });
        event.account_id
    }

    fn create_bank_account_impl(
        &self,
        account_number: String,
        description: String,
    ) -> Result<BankAccountCreated, CommandError> {
        if self.has_account(&account_number) {
            Err(CommandError::Validation(format!(
                "Account {} already exists",
                account_number
            )))
        } else {
            Ok(BankAccountCreated {
                budget_id: self.id,
                account_id: Uuid::new_v4(),
                account_number,
                description,
            })
        }
    }
}
