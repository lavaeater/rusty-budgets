use crate::models::{Money, PeriodId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeriodSummary {
    pub period_id: PeriodId,
    /// Actual income received this period (positive).
    pub income_actual: Money,
    /// Actual expenses + savings this period (positive).
    pub expense_actual: Money,
    /// net = income_actual - expense_actual (negative = deficit).
    pub net: Money,
    /// Cumulative net from the earliest period up to and including this one.
    pub running_net: Money,
}
