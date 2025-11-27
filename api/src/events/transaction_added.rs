use std::hash::Hash;
use serde::{Deserialize, Serialize};
use cqrs_macros::DomainEvent;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use dioxus::logger::tracing;
use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{get_transaction_hash, BankTransaction, PeriodId};
use crate::models::Budget;
use crate::models::Money;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct TransactionAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub transaction_id: Uuid,
    pub account_number: String,
    pub amount: Money,
    pub balance: Money,
    pub description: String,
    pub date: DateTime<Utc>,
}

impl Hash for TransactionAdded {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.amount.hash(state);
        self.account_number.hash(state);
        self.description.hash(state);
        self.date.hash(state);
    }
}

impl TransactionAddedHandler for Budget {
    fn apply_add_transaction(&mut self, event: &TransactionAdded) -> Uuid {
        let period_id = PeriodId::from_date(event.date, self.month_begins_on());
        self.with_period_mut(period_id).transactions.insert(BankTransaction::new(
            event.transaction_id,
            &event.account_number,
            event.amount,
            event.balance,
            &event.description,
            event.date,
        ));
        event.transaction_id
    }

    fn add_transaction_impl(
        &self,
        account_number: String,
        amount: Money,
        balance: Money,
        description: String,
        date: DateTime<Utc>,
    ) -> Result<TransactionAdded, CommandError> {
        let hash = get_transaction_hash(&amount, &balance, &account_number, &description, &date);
        
        if self.can_insert_transaction(&hash) {
            Ok(TransactionAdded {
                budget_id: self.id,
                account_number,
                transaction_id: Uuid::new_v4(),
                amount,
                balance,
                description,
                date,
            })
        } else {
            Err(CommandError::Validation("Transaction already exists.".to_string()))
        }
    }
}