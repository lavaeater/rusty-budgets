use crate::models::bank_transaction_store::BankTransactionStore;
use crate::models::budget_period_id::PeriodId;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::models::{
    ActualItem, BudgetItem, BudgetingType, BudgetingTypeOverview, MatchRule, Money,
};
use core::fmt::Display;
use dioxus::logger::tracing;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeStruct;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{HashMap, HashSet};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BudgetPeriod {
    pub id: PeriodId,
    pub actual_items: HashMap<Uuid, ActualItem>,
    pub transactions: BankTransactionStore,
}

impl Serialize for BudgetPeriod {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("BudgetPeriod", 3)?;
        state.serialize_field("id", &self.id)?;
        let actual_items: HashMap<String, ActualItem> = self
            .actual_items
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect();
        state.serialize_field("actual_items", &actual_items)?;
        state.serialize_field("transactions", &self.transactions)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for BudgetPeriod {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Id,
            ActualItems,
            Transactions,
        }

        struct BudgetPeriodVisitor;

        impl<'de> Visitor<'de> for BudgetPeriodVisitor {
            type Value = BudgetPeriod;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct BudgetPeriod")
            }

            fn visit_map<V>(self, mut map: V) -> Result<BudgetPeriod, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut id = None;
                let mut actual_items = None;
                let mut transactions = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Id => {
                            if id.is_some() {
                                return Err(de::Error::duplicate_field("id"));
                            }
                            id = Some(map.next_value()?);
                        }
                        Field::ActualItems => {
                            if actual_items.is_some() {
                                return Err(de::Error::duplicate_field("actual_items"));
                            }
                            actual_items = Some(map.next_value()?);
                        }
                        Field::Transactions => {
                            if transactions.is_some() {
                                return Err(de::Error::duplicate_field("transactions"));
                            }
                            transactions = Some(map.next_value()?);
                        }
                    }
                }

                let id = id.ok_or_else(|| de::Error::missing_field("id"))?;
                let actual_items =
                    actual_items.ok_or_else(|| de::Error::missing_field("actual_items"))?;
                let transactions =
                    transactions.ok_or_else(|| de::Error::missing_field("transactions"))?;

                Ok(BudgetPeriod {
                    id,
                    actual_items,
                    transactions,
                })
            }
        }

        const FIELDS: &[&str] = &["id", "actual_items", "transactions"];
        deserializer.deserialize_struct("BudgetPeriod", FIELDS, BudgetPeriodVisitor)
    }
}

impl BudgetPeriod {
    pub fn mutate_actual(&mut self, actual_id: Uuid, mut mutator: impl FnMut(&mut ActualItem)) {
        if let Some(actual) = self.get_actual_mut(actual_id) {
            mutator(actual);
        }
    }
    pub fn get_actual(&self, id: Uuid) -> Option<&ActualItem> {
        self.actual_items.get(&id)
    }

    pub fn get_actual_mut(&mut self, id: Uuid) -> Option<&mut ActualItem> {
        self.actual_items.get_mut(&id)
    }

    pub fn add_actual(&mut self, actual_item: ActualItem) {
        self.actual_items.insert(actual_item.id, actual_item);
    }
    pub fn contains_actual_for_item(&self, item_id: Uuid) -> bool {
        self.actual_items
            .values()
            .any(|i| i.budget_item_id == item_id)
    }

    pub fn contains_actual(&self, actual_id: Uuid) -> bool {
        self.actual_items.contains_key(&actual_id)
    }
    
    fn clear_hashmaps_and_transactions(&mut self) {
        self.transactions.clear();
    }
    pub fn clone_to(&self, id: PeriodId) -> Self {
        let mut period = self.clone();
        period.id = id;
        period.clear_hashmaps_and_transactions();
        period
    }
    pub fn new_for(id: PeriodId) -> Self {
        let mut period = Self {
            id,
            actual_items: HashMap::new(),
            transactions: BankTransactionStore::default(),
        };
        period.clear_hashmaps_and_transactions();
        period
    }

    pub fn evaluate_rules(&self, rules: &HashSet<MatchRule>) -> Vec<(Uuid, Uuid)> {
        let mut matched_transactions = Vec::new();
        let items = self.actual_items.values().collect();
        for transaction in self.transactions.list_transactions_for_connection() {
            for rule in rules {
                if rule.matches_transaction(&transaction) {
                    if let Some(item_id) = self.get_item_for_rule(rule, &items) {
                        matched_transactions.push((transaction.id, item_id));
                        break;
                    }
                }
            }
        }
        matched_transactions
    }

    pub fn get_item_for_rule(&self, rule: &MatchRule, items: &Vec<&ActualItem>) -> Option<Uuid> {
        items.iter().find(|i| rule.matches_item(i)).map(|i| i.id)
    }
    
    pub fn all_actual_items(&self) -> Vec<ActualItem> {
        self.actual_items.values().cloned().collect()
    }
}
