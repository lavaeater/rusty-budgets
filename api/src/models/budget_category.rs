use std::fmt::Display;
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash, Eq)]
pub enum BudgetCategory {
    Income(String),
    Expense(String),
    Savings(String),
}

impl Default for BudgetCategory {
    fn default() -> Self {
        BudgetCategory::Expense("Övrigt".to_string())
    }
}

impl Display for BudgetCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetCategory::Income(s) => write!(f, "Income({})", s),
            BudgetCategory::Expense(s) => write!(f, "Expense({})", s),
            BudgetCategory::Savings(s) => write!(f, "Savings({})", s),
        }
    }
}

impl FromStr for BudgetCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Basic format: "VariantName(value)"
        if let Some(rest) = s.strip_prefix("Income(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Income(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("Expense(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Expense(rest.to_string()));
        } else if let Some(rest) = s.strip_prefix("Savings(").and_then(|s| s.strip_suffix(")")) {
            return Ok(BudgetCategory::Savings(rest.to_string()));
        }
        Err(format!("Unknown BudgetCategory format: {}", s))
    }
}