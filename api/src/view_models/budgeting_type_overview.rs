use serde::{Deserialize, Serialize};
use crate::models::{BudgetingType, Money};

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Default, Hash, PartialEq, Eq)]
pub struct BudgetingTypeOverview {
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub is_ok: bool,
}