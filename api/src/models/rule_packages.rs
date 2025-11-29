use serde::{Deserialize, Serialize};
use crate::models::BudgetingType;
use crate::models::BudgetingType::{Expense, Income, Savings};
use crate::view_models::Rule;
use crate::view_models::Rule::{Difference, SelfDiff, Sum};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackages {
    pub rule_packages: Vec<RulePackage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePackage {
    pub budgeting_type: BudgetingType,
    pub budgeted_rule: Rule,
    pub actual_rule: Rule,
    pub remaining_rule: Rule,
}

impl RulePackage {
    pub fn new(
        budgeting_type: BudgetingType,
        budgeted_rule: Rule,
        actual_rule: Rule,
        remaining_rule: Rule,
    ) -> Self {
        Self {
            budgeting_type,
            budgeted_rule,
            actual_rule,
            remaining_rule,
        }
    }
}

impl Default for RulePackages {
    fn default() -> Self {
        Self {
            rule_packages: vec![
                RulePackage::new(
                    Income,
                    Sum(vec![Income]),
                    Sum(vec![Income]),
                    Difference(Income, vec![Expense, Savings]),
                ),
                RulePackage::new(
                    Expense,
                    Sum(vec![Expense]),
                    Sum(vec![Expense]),
                    SelfDiff(Expense),
                ),
                RulePackage::new(
                    Savings,
                    Sum(vec![Savings]),
                    Sum(vec![Savings]),
                    SelfDiff(Savings),
                ),
            ],
        }
    }
}