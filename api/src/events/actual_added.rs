use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{Budget, BudgetItem, BudgetingType, ActualItem, Money, BudgetPeriodId};

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ActualAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub actual_id: Uuid,
    pub item_id: Uuid,
    pub period_id: BudgetPeriodId,
    pub budgeted_amount: Money,
}

impl ActualAddedHandler for Budget {
    fn apply_add_actual(&mut self, event: &ActualAdded) -> Uuid {
        let budget_item = self.budget_items.get(&event.item_id).unwrap();
        
        let new_actual = ActualItem::new(
            event.actual_id,
            budget_item.clone(),
            event.period_id,
            event.budgeted_amount,
            Money::default(),
            None,
            Vec::new(),
        );
        
        self.with_period_mut(event.period_id).add_actual(new_actual);
        
        event.actual_id
    }

    fn add_actual_impl(
        &self,
        item_id: Uuid,
        period_id: BudgetPeriodId,
        budgeted_amount: Money,
    ) -> Result<ActualAdded, CommandError> {
        if self.with_period(period_id).contains_actual_for_item(item_id) {
            Err(CommandError::Validation("Item already exists."))
        } else {
            Ok(ActualAdded {
                budget_id: self.id,
                actual_id: Uuid::new_v4(),
                item_id,
                period_id,
                budgeted_amount,
            })
        }
    }
}
