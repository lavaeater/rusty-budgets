use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, PeriodId};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Chooses the month from which category balances start carrying forward.
///
/// Carryover is opt-in and dated rather than global because the log holds two
/// years of history from before the budget was kept properly — most early
/// `ActualItem`s have `budgeted_amount == 0`, so replaying carryover across all
/// of it would compound spending-with-no-budget into large meaningless
/// balances. `None` disables carryover entirely, which is the pre-existing
/// behaviour and therefore the default for every existing budget.
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct CarryoverConfigured {
    pub budget_id: Uuid,
    /// First period that carries a balance forward. Everything before it is
    /// treated as zero carryover.
    pub from_period: Option<PeriodId>,
}

impl CarryoverConfiguredHandler for Budget {
    fn apply_configure_carryover(&mut self, event: &CarryoverConfigured) -> Uuid {
        self.carryover_from = event.from_period;
        self.id
    }

    fn configure_carryover_impl(
        &self,
        from_period: Option<PeriodId>,
    ) -> Result<CarryoverConfigured, CommandError> {
        Ok(CarryoverConfigured {
            budget_id: self.id,
            from_period,
        })
    }
}
