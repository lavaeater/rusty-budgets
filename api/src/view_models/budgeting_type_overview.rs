use crate::models::{BudgetingType, Money};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Default, Hash, PartialEq, Eq)]
pub struct BudgetingTypeOverview {
    pub budgeting_type: BudgetingType,
    pub budgeted_amount: Money,
    pub actual_amount: Money,
    pub remaining_budget: Money,
    pub is_ok: bool,
}
