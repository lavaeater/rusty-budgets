use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemStatus {
    Balanced,
    OverBudget,
    #[default]
    NotBudgeted,
    UnderBudget,
}