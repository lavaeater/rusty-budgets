use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A lightweight entry in a user's budget list — enough to show and pick
/// between budgets without loading each one in full. See
/// `BudgetCommandsTrait::list_budgets`/`AsyncBudgetCommandsTrait::list_budgets`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BudgetSummary {
    pub id: Uuid,
    pub name: String,
    /// Whether this is the budget that loads by default — the one
    /// `switch_budget` changes.
    pub default: bool,
}
