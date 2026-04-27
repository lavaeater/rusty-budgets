use crate::models::{Money, Periodicity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Per-tag spending summary, computed from all historical transaction data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagSummary {
    pub tag_id: Uuid,
    pub name: String,
    pub periodicity: Periodicity,
    /// Average monthly amount (signed — negative for expenses, positive for income).
    pub average_monthly: Money,
    /// Average yearly amount (average_monthly × 12).
    pub average_yearly: Money,
    /// Number of tagged transactions included in the average.
    pub transaction_count: u32,
}
