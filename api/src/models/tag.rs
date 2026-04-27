use crate::models::budget_item::Periodicity;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tag {
    pub id: Uuid,
    pub name: String,
    pub periodicity: Periodicity,
    pub deleted: bool,
}

impl Tag {
    pub fn new(id: Uuid, name: String, periodicity: Periodicity) -> Self {
        Self {
            id,
            name,
            periodicity,
            deleted: false,
        }
    }
}
