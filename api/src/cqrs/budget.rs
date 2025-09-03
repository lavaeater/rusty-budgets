use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use joydb::Model;
use crate::cqrs::budgets::{BankTransaction, BudgetGroup, BudgetItem};
use crate::cqrs::framework::Aggregate;

// --- Budget Domain ---
#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,
    pub budget_groups: HashMap<Uuid, BudgetGroup>,
    pub bank_transactions: HashMap<Uuid, BankTransaction>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub default_budget: bool,
    pub last_event: i64,
    pub version: u64,
}

impl Budget {
    pub(crate) fn new(id: Uuid, name: impl Into<String>, created_by: Uuid, default_budget: bool) -> Self {
        Self {
            id,
            name: name.into(),
            user_id: created_by,
            default_budget,
            budget_groups: HashMap::new(),
            bank_transactions: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_event: 0,
            version: 0,
        }
    }
}

impl Budget {
    pub fn get_item_mut(&mut self, item_id: &Uuid) -> Option<&mut BudgetItem> {
        self.budget_groups
            .iter_mut()
            .flat_map(move |(_, group)| group.items.iter_mut())
            .find(|item| item.id == *item_id)
    }

    pub fn get_item(&self, item_id: &Uuid) -> Option<&BudgetItem> {
        self.budget_groups
            .iter()
            .flat_map(move |(_, group)| group.items.iter())
            .find(|item| item.id == *item_id)
    }
}

// --- Aggregate implementation ---
impl Aggregate for Budget {
    type Id = Uuid;

    fn _new(id: Self::Id) -> Self {
        Self {
            id,
            name: String::new(),
            user_id: Uuid::new_v4(),
            default_budget: false,
            budget_groups: HashMap::new(),
            bank_transactions: HashMap::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_event: 0,
            version: 0,
        }
    }

    fn update_timestamp(&mut self, timestamp: i64, updated_at: DateTime<Utc>) {
        if self.last_event < timestamp {
            self.last_event = timestamp;
            self.updated_at = updated_at;
            if self.version == 0 {
                self.created_at = updated_at;
            }
            self.version += 1;
        } else {
            panic!("Event timestamp is older than last event timestamp");
        }
    }

    fn _version(&self) -> u64 {
        self.version
    }
}