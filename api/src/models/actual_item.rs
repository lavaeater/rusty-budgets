use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::models::{BudgetItem, PeriodId, Money};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActualItem {
    pub id: Uuid,
    #[serde(
        serialize_with = "actual_item_arc_mutex_serde::serialize",
        deserialize_with = "actual_item_arc_mutex_serde::deserialize"
    )]
    pub budget_item: Arc<Mutex<BudgetItem>>,
    pub period_id: PeriodId,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

impl ActualItem {
    pub fn new(
        id: Uuid,
        budget_item: Arc<Mutex<BudgetItem>>,
        period_id: PeriodId,
        budgeted_amount: Money,
        actual_amount: Money,
        notes: Option<String>,
        tags: Vec<String>,
    ) -> ActualItem {
        ActualItem {
            id,
            budget_item,
            period_id,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        }
    }
}

mod actual_item_arc_mutex_serde {
    use crate::models::budget_period::BudgetPeriod;
    use crate::models::budget_period_id::PeriodId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use crate::models::BudgetItem;

    pub fn serialize<S>(
        budget_item: &Arc<Mutex<BudgetItem>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        budget_item.lock().unwrap().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<Mutex<BudgetItem>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let budget_item = BudgetItem::deserialize(deserializer)?;
        Ok(Arc::new(Mutex::new(budget_item)))
    }
}