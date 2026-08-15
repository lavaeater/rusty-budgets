use crate::models::Money;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A rule match the user still has to confirm.
///
/// Produced for [`crate::models::Matching::Suggest`] tags — card spending,
/// where a payee maps to a category most of the time but not always. Carries
/// the transaction detail so the UI can show what is being proposed without a
/// second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TagSuggestion {
    pub tx_id: Uuid,
    pub tag_id: Uuid,
    pub tag_name: String,
    pub description: String,
    pub amount: Money,
    pub date: DateTime<Utc>,
}
