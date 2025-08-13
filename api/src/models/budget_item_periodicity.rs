use std::fmt::Display;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BudgetItemPeriodicity {
    Once,
    #[default]
    Monthly,
    Quarterly,
    Yearly,
}

impl Display for BudgetItemPeriodicity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetItemPeriodicity::Once => write!(f, "Once"),
            BudgetItemPeriodicity::Monthly => write!(f, "Monthly"),
            BudgetItemPeriodicity::Quarterly => write!(f, "Quarterly"),
            BudgetItemPeriodicity::Yearly => write!(f, "Yearly"),
        }
    }
}