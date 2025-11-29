use crate::models::{BudgetItem, BudgetingType, Money, PeriodId};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualItem {
    pub id: Uuid,
    pub item_name: String,
    pub budget_item_id: Uuid,
    pub budgeting_type: BudgetingType,
    pub period_id: PeriodId,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

impl PartialEq for ActualItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.budget_item_id == other.budget_item_id
            && self.budgeting_type == other.budgeting_type  
            && self.period_id == other.period_id
            && self.budgeted_amount == other.budgeted_amount
            && self.actual_amount == other.actual_amount
            && self.notes == other.notes
            && self.tags == other.tags
    }
}

impl ActualItem {
    pub fn new(
        id: Uuid,
        item_name: &str,
        budget_item_id: Uuid,
        budgeting_type: BudgetingType,
        period_id: PeriodId,
        budgeted_amount: Money,
        actual_amount: Money,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> ActualItem {
        ActualItem {
            id,
            item_name: item_name.to_string(),
            budget_item_id,
            budgeting_type,
            period_id,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        }
    }
}

#[cfg(test)]
pub mod actual_item_tests {
    use crate::models::{ActualItem, BudgetingType, Currency};
    use crate::models::BudgetItem;
    use crate::models::Money;
    use crate::models::PeriodId;
    use std::sync::Arc;
    use std::sync::Mutex;
    use uuid::Uuid;

    #[test]
    pub fn serde_round_trip() {
        let budget_item = BudgetItem::new(
            Uuid::new_v4(),
            "Test Budget Item",
            BudgetingType::Expense
        );
        let actual_item = ActualItem::new(
            Uuid::new_v4(),
            &budget_item.name,
            budget_item.id,
            budget_item.budgeting_type,
            PeriodId::new(2025,12),
            Money::new_dollars(1000, Currency::USD),
            Money::new_dollars(1000, Currency::USD),
            None,
            vec![],
        );
        let serialized = serde_json::to_string(&actual_item).unwrap();
        let deserialized: ActualItem = serde_json::from_str(&serialized).unwrap();
        assert_eq!(actual_item, deserialized);
        assert_eq!(actual_item.budgeting_type, deserialized.budgeting_type);
    }
}
