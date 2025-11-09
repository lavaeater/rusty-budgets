use crate::models::{BudgetItem, BudgetingType, Money, PeriodId};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ActualItem {
    pub id: Uuid,
    pub budget_item_id: Uuid,
    pub budget_item: Arc<Mutex<BudgetItem>>,
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
            && self.period_id == other.period_id
            && self.budgeted_amount == other.budgeted_amount
            && self.actual_amount == other.actual_amount
            && self.notes == other.notes
            && self.tags == other.tags
    }
}

impl Serialize for ActualItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ActualItem", 7)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("budget_item_id", &self.budget_item_id)?;
        state.serialize_field("budget_item", &self.budget_item.lock().unwrap().clone())?;
        state.serialize_field("period_id", &self.period_id)?;
        state.serialize_field("budgeted_amount", &self.budgeted_amount)?;
        state.serialize_field("actual_amount", &self.actual_amount)?;
        state.serialize_field("notes", &self.notes)?;
        state.serialize_field("tags", &self.tags)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ActualItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            BudgetItemId,
            BudgetItem,
            PeriodId,
            BudgetedAmount,
            ActualAmount,
            Notes,
            Tags,
        }

        struct ActualItemVisitor;

        impl<'de> Visitor<'de> for ActualItemVisitor {
            type Value = ActualItem;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct ActualItem")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ActualItem, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut budget_item_id = None;
                let mut budget_item = None;
                let mut period_id = None;
                let mut budgeted_amount = None;
                let mut actual_amount = None;
                let mut notes = None;
                let mut tags = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::BudgetItemId => {
                            if budget_item_id.is_some() {
                                return Err(de::Error::duplicate_field("budget_item_id"));
                            }
                            budget_item_id = Some(map.next_value()?);
                        }
                        Field::BudgetItem => {
                            if budget_item.is_some() {
                                return Err(de::Error::duplicate_field("budget_item"));
                            }
                            budget_item = Some(map.next_value()?);
                        }
                        Field::PeriodId => {
                            if period_id.is_some() {
                                return Err(de::Error::duplicate_field("period_id"));
                            }
                            period_id = Some(map.next_value()?);
                        }
                        Field::BudgetedAmount => {
                            if budgeted_amount.is_some() {
                                return Err(de::Error::duplicate_field("budgeted_amount"));
                            }
                            budgeted_amount = Some(map.next_value()?);
                        }
                        Field::ActualAmount => {
                            if actual_amount.is_some() {
                                return Err(de::Error::duplicate_field("actual_amount"));
                            }
                            actual_amount = Some(map.next_value()?);
                        }
                        Field::Notes => {
                            if notes.is_some() {
                                return Err(de::Error::duplicate_field("notes"));
                            }
                            notes = Some(map.next_value()?);
                        }
                        Field::Tags => {
                            if tags.is_some() {
                                return Err(de::Error::duplicate_field("tags"));
                            }
                            tags = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let budget_item_id =
                    budget_item_id.ok_or_else(|| de::Error::missing_field("budget_item_id"))?;
                let budget_item: BudgetItem =
                    budget_item.ok_or_else(|| de::Error::missing_field("budget_item"))?;
                let period_id = period_id.ok_or_else(|| de::Error::missing_field("period_id"))?;
                let budgeted_amount =
                    budgeted_amount.ok_or_else(|| de::Error::missing_field("budgeted_amount"))?;
                let actual_amount =
                    actual_amount.ok_or_else(|| de::Error::missing_field("actual_amount"))?;
                let notes = notes.ok_or_else(|| de::Error::missing_field("notes"))?;
                let tags = tags.ok_or_else(|| de::Error::missing_field("tags"))?;

                Ok(ActualItem {
                    id,
                    budget_item_id,
                    budget_item: Arc::new(Mutex::new(budget_item)),
                    period_id,
                    budgeted_amount,
                    actual_amount,
                    notes,
                    tags,
                })
            }
        }

        const FIELDS: &[&str] = &[
            "id",
            "budget_item_id",
            "budget_item",
            "period_id",
            "budgeted_amount",
            "actual_amount",
            "notes",
            "tags",
        ];
        deserializer.deserialize_struct("ActualItem", FIELDS, ActualItemVisitor)
    }
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
        let budget_item_id = budget_item.lock().unwrap().id;
        ActualItem {
            id,
            budget_item_id,
            budget_item: budget_item.clone(),
            period_id,
            budgeted_amount,
            actual_amount,
            notes,
            tags,
        }
    }
    pub fn budgeting_type(&self)-> BudgetingType {
        self.budget_item.lock().unwrap().budgeting_type
    }
    
    pub fn item_name(&self) -> &str {
        &self.budget_item.lock().unwrap().name
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
        let budget_item = Arc::new(Mutex::new(BudgetItem::new(
            Uuid::new_v4(),
            "Test Budget Item",
            BudgetingType::Expense
        )));
        let actual_item = ActualItem::new(
            Uuid::new_v4(),
            budget_item.clone(),
            PeriodId::new(2025,12),
            Money::new_dollars(1000, Currency::USD),
            Money::new_dollars(1000, Currency::USD),
            None,
            vec![],
        );
        let serialized = serde_json::to_string(&actual_item).unwrap();
        let deserialized: ActualItem = serde_json::from_str(&serialized).unwrap();
        assert_eq!(actual_item, deserialized);
        assert_eq!(actual_item.budgeting_type(), deserialized.budgeting_type());
        assert_eq!(actual_item.budget_item.lock().unwrap().clone(), deserialized.budget_item.lock().unwrap().clone());
    }
}
