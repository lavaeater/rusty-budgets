use crate::models::budgeting_type::BudgetingType;
use crate::models::money::Money;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Arc;
use strum::IntoEnumIterator;
use uuid::Uuid;

/// The store
#[derive(Default, Debug, Clone)]
pub struct BudgetItemStore {
    items: HashMap<Uuid, Arc<BudgetItem>>,
    by_type: HashMap<BudgetingType, Vec<Arc<BudgetItem>>>,
    items_and_types: HashMap<Uuid, BudgetingType>,
}

impl Serialize for BudgetItemStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("BudgetItemStore", 3)?;

        // Serialize items by dereferencing the Arc to get the inner BudgetItem
        let items_map: HashMap<Uuid, &BudgetItem> =
            self.items.iter().map(|(k, v)| (*k, v.as_ref())).collect();
        state.serialize_field("items", &items_map)?;

        // Serialize by_type by dereferencing the Arc in each Vec
        let by_type_map: HashMap<BudgetingType, Vec<&BudgetItem>> = self
            .by_type
            .iter()
            .map(|(k, v)| (*k, v.iter().map(|arc| arc.as_ref()).collect()))
            .collect();
        state.serialize_field("by_type", &by_type_map)?;
        state.serialize_field("items_and_types", &self.items_and_types)?;

        state.end()
    }
}

impl<'de> Deserialize<'de> for BudgetItemStore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Items,
            ByType,
            ItemsAndTypes,
        }

        struct BudgetItemStoreVisitor;

        impl<'de> Visitor<'de> for BudgetItemStoreVisitor {
            type Value = BudgetItemStore;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BudgetItemStore")
            }

            fn visit_map<V>(self, mut map: V) -> Result<BudgetItemStore, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut items: Option<HashMap<Uuid, BudgetItem>> = None;
                let mut by_type: Option<HashMap<BudgetingType, Vec<BudgetItem>>> = None;
                let mut items_and_types: Option<HashMap<Uuid, BudgetingType>> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Items => {
                            if items.is_some() {
                                return Err(de::Error::duplicate_field("items"));
                            }
                            items = Some(map.next_value()?);
                        }
                        Field::ByType => {
                            if by_type.is_some() {
                                return Err(de::Error::duplicate_field("by_type"));
                            }
                            by_type = Some(map.next_value()?);
                        }
                        Field::ItemsAndTypes => {
                            if items_and_types.is_some() {
                                return Err(de::Error::duplicate_field("items_and_types"));
                            }
                            items_and_types = Some(map.next_value()?);
                        }
                    }
                }

                let items = items.ok_or_else(|| de::Error::missing_field("items"))?;
                let by_type = by_type.ok_or_else(|| de::Error::missing_field("by_type"))?;
                let items_and_types =
                    items_and_types.ok_or_else(|| de::Error::missing_field("items_and_types"))?;

                // Convert the deserialized data to the expected format with Arc
                let items_with_arc: HashMap<Uuid, Arc<BudgetItem>> =
                    items.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();

                let by_type_with_arc: HashMap<BudgetingType, Vec<Arc<BudgetItem>>> = by_type
                    .into_iter()
                    .map(|(k, v)| {
                        let arcs: Vec<Arc<BudgetItem>> = v.into_iter().map(Arc::new).collect();
                        (k, arcs)
                    })
                    .collect();

                Ok(BudgetItemStore {
                    items: items_with_arc,
                    by_type: by_type_with_arc,
                    items_and_types,
                })
            }
        }

        const FIELDS: &'static [&'static str] = &["items", "by_type", "items_and_types"];
        deserializer.deserialize_struct("BudgetItemStore", FIELDS, BudgetItemStoreVisitor)
    }
}

impl BudgetItemStore {
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn insert(&mut self, item: &BudgetItem, item_type: BudgetingType) -> bool {
        if let Entry::Vacant(e) = self.items.entry(item.id) {
            let arc = Arc::new(item.clone());
            e.insert(arc.clone());
            self.by_type.entry(item_type).or_default().push(arc);
            self.items_and_types.insert(item.id, item_type);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: Uuid) -> Option<BudgetItem> {
        if self.items.contains_key(&id) {
            let arc = self.items.remove(&id).unwrap();
            if let Some(item_type) = self.items_and_types.remove(&id) {
                self.by_type.entry(item_type).and_modify(|v| {
                    v.retain(|x| x.id != id);
                });
            }
            Some(Arc::try_unwrap(arc).unwrap())
        } else {
            None
        }
    }

    pub fn change_type(&mut self, id: &Uuid, budgeting_type: BudgetingType) {
        if let Some(item) = self.remove(*id) {
            self.insert(&item, budgeting_type);
        }
    }

    pub fn type_for(&self, id: &Uuid) -> Option<&BudgetingType> {
        self.items_and_types.get(id)
    }

    pub fn by_type(&self, budgeting_type: &BudgetingType) -> Option<Vec<&BudgetItem>> {
        self.by_type
            .get(budgeting_type)
            .map(|items| items.iter().map(|arc| arc.as_ref()).collect())
    }

    pub fn items_by_type(&self) -> Vec<(usize, BudgetingType, Vec<BudgetItem>)> {
        BudgetingType::iter()
            .enumerate()
            .map(|(index, t)| (index, t, self.by_type(&t).unwrap_or_default().into_iter().cloned().collect()))
            .collect::<Vec<_>>()
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut BudgetItem> {
        self.items
            .get_mut(id)
            .and_then(|arc| Some(Arc::make_mut(arc)))
    }

    pub fn get(&self, id: &Uuid) -> Option<&BudgetItem> {
        self.items.get(id).and_then(|arc| Some(arc.as_ref()))
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        self.items.contains_key(id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub budgeted_amount: Money,
    pub spent_amount: Money,
    pub notes: Option<String>,
    pub tags: Vec<String>,
}

impl BudgetItem {
    pub fn new(
        id: Uuid,
        name: &str,
        budgeted_amount: Money,
        notes: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            budgeted_amount,
            spent_amount: Money::new_dollars(0, budgeted_amount.currency()),
            notes,
            tags: tags.unwrap_or_default(),
        }
    }
}
