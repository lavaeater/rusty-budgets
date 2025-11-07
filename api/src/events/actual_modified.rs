use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, PeriodId};
use crate::models::BudgetingType;
use crate::models::Money;
use cqrs_macros::DomainEvent;
use dioxus::logger::tracing;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualModified {
    pub budget_id: Uuid,
    pub actual_id: Uuid,
    pub period_id: PeriodId,
    pub budgeted_amount: Option<Money>,
    pub actual_amount: Option<Money>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

impl ActualModifiedHandler for Budget {
    fn apply_modify_actual(&mut self, event: &ActualModified) -> Uuid {
        self.with_period_mut(event.period_id)
            .mutate_actual(event.actual_id, |actual| {
                event.budgeted_amount.map(|budgeted_amount| actual.budgeted_amount = budgeted_amount);
                event.actual_amount.map(|actual_amount| actual.actual_amount = actual_amount);
                actual.notes = event.notes;
                event.tags.map(|tags| actual.tags = tags);
            });
        event.actual_id
    }

    fn modify_actual_impl(
        &self,
        actual_id: Uuid,
        period_id: PeriodId,
        budgeted_amount: Option<Money>,
        actual_amount: Option<Money>,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<ActualModified, CommandError> {
        if !self.with_period(period_id).contains_actual(actual_id) {
            tracing::error!("Actual not found");
            Err(CommandError::NotFound("Actual not found".to_string()))
        } else {
            Ok(ActualModified {
                budget_id: self.id,
                actual_id,
                period_id,
                budgeted_amount,
                actual_amount,
                notes,
                tags,
            })
        }
    }
}
