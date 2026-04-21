use crate::models::{ActualItem, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueKind {
    Budgeted,
    Spent,
}

impl ValueKind {
    pub fn pick(&self, item: &ActualItem) -> Money {
        match self {
            ValueKind::Budgeted => item.budgeted_amount,
            ValueKind::Spent => item.actual_amount,
        }
    }
}
