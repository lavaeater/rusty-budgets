use core::fmt;
use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};

#[derive(Default,Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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