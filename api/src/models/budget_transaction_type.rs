use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum BudgetTransactionType {
    #[default]
    StartValue,
    Adjustment,
}