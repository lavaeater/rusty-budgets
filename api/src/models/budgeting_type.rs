use core::fmt;
use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(Default,Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, EnumIter)]
pub enum BudgetingType {
    #[default]
    Income,
    Expense,
    Savings,
}

impl Display for BudgetingType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BudgetingType::Income => "Inkomst",
                BudgetingType::Expense => "Utgift",
                BudgetingType::Savings => "Sparande",
            }
        )
    }
}