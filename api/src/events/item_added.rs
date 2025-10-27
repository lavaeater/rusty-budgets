use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::Budget;
use crate::models::BudgetItem;
use crate::models::BudgetingType;
use crate::models::BudgetPeriodId;
use crate::models::Money;
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget")]
pub struct ItemAdded {
    pub budget_id: Uuid,
    #[event_id]
    pub item_id: Uuid,
    pub name: String,
    pub item_type: BudgetingType,
    pub budgeted_amount: Money,
    pub tx_id: Option<Uuid>
}

impl ItemAddedHandler for Budget {
    fn apply_add_item(&mut self, event: &ItemAdded) -> Uuid {
        let new_item = BudgetItem::new(
            event.item_id,
            &event.name,
            event.budgeted_amount,
            None,
            None,
        );
        let period = if let Some(tx_id) = event.tx_id {
            self.get_transaction(&tx_id).map(|transaction| BudgetPeriodId::from_date(transaction.date, *self.month_begins_on()))         
        } else {
            None
        };
        let new_item_id = new_item.id;
        if let Some(period) = period {
            self.with_period_mut(&period, |bp| bp.budget_items.insert(&new_item, event.item_type));
        } else {
            self.with_current_period_mut(|bp| bp.budget_items.insert(&new_item, event.item_type));
        }
        new_item_id
    }

    fn add_item_impl(
        &self,
        name: String,
        item_type: BudgetingType,
        budgeted_amount: Money,
        tx_id: Option<Uuid>
    ) -> Result<ItemAdded, CommandError> {
        if self.contains_item_with_name(&name) {
            return Err(CommandError::Validation("Item already exists."));
        }
        Ok(ItemAdded {
            budget_id: self.id,
            item_id: Uuid::new_v4(),
            name,
            item_type,
            budgeted_amount,
            tx_id
        })
    }
}
