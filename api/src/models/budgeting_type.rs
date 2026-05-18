use core::fmt;
use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;
#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    Serialize,
    Deserialize,
    PartialEq,
    Eq,
    Hash,
    EnumIter,
    PartialOrd,
    Ord,
)]
pub enum BudgetingType {
    #[default]
    Income = 1,
    Expense = 2,
    Savings = 3,
    InternalTransfer = 4,
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
                BudgetingType::InternalTransfer => "Intern överföring",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_variants() {
        assert_eq!(BudgetingType::Income.to_string(), "Inkomst");
        assert_eq!(BudgetingType::Expense.to_string(), "Utgift");
        assert_eq!(BudgetingType::Savings.to_string(), "Sparande");
        assert_eq!(BudgetingType::InternalTransfer.to_string(), "Intern överföring");
    }

    #[test]
    fn default_is_income() {
        assert_eq!(BudgetingType::default(), BudgetingType::Income);
    }

    #[test]
    fn ordering() {
        assert!(BudgetingType::Income < BudgetingType::Expense);
        assert!(BudgetingType::Expense < BudgetingType::Savings);
        assert!(BudgetingType::Savings < BudgetingType::InternalTransfer);
    }
}
