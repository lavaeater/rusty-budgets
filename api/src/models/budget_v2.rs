use std::fmt::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use joydb::Model;
use uuid::Uuid;

/// A simplified, more intuitive budget model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,

    /// Total income available for budgeting (single pool)
    pub total_income: f32,

    /// Organized budget items by group
    pub budget_groups: HashMap<String, BudgetGroup>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub default_budget: bool,
}

/// A group of related budget items (e.g., "Household", "Utilities", "Entertainment")
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetGroup {
    pub id: Uuid,
    pub name: String,
    pub items: Vec<BudgetItem>,
}

/// Individual budget item (expense or savings)
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct BudgetItem {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,

    /// Amount allocated to this item from income pool
    pub budgeted_amount: f32,

    /// Actual spending tracked from bank transactions
    pub actual_spent: f32,
    /// Optional notes or description
    pub notes: Option<String>,
}

impl PartialEq for BudgetItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.item_type == other.item_type
            && self.budgeted_amount == other.budgeted_amount
            && self.actual_spent == other.actual_spent
            && (
            match &self.notes {
                None => {
                    match other.notes {
                        None => true,
                        _ => false,
                    }
                }
                Some(self_id) => match &other.notes {
                    None => false,
                    Some(other_id) => self_id == other_id,
                },
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetItemType {
    Expense,
    Savings,
}

/// Represents a bank transaction that affects budget items
#[derive(Debug, Clone, Serialize, Deserialize, Model)]
pub struct BankTransaction {
    pub id: Uuid,
    pub amount: f32,
    pub description: String,
    pub date: chrono::NaiveDate,
    pub budget_item_id: Option<Uuid>, // Which budget item this affects
}

impl PartialEq for BankTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.amount == other.amount
            && self.description == other.description
            && self.date == other.date
            && (
            match self.budget_item_id {
                None => {
                    match other.budget_item_id {
                        None => true,
                        _ => false,
                    }
                }
                    Some(self_id) => match other.budget_item_id {
                        None => false,
                        Some(other_id) => self_id == other_id,
                    },
            })
    }
}

/// Summary of budget status and issues requiring attention
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetSummary {
    pub id: Uuid,
    pub name: String,
    pub total_income: f32,
    pub total_budgeted: f32,
    pub total_spent: f32,
    pub is_balanced: bool,
    pub unallocated_income: f32,
    pub issues: Vec<BudgetIssue>,
    pub default_budget: bool,
}

/// Issues that require user attention
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetIssue {
    pub issue_type: BudgetIssueType,
    pub item_id: Uuid,
    pub item_name: String,
    pub group_name: String,
    pub amount: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetIssueType {
    /// Item has been overspent (actual > budgeted)
    Overspent,
    /// Budget is not balanced (total budgeted != total income)
    Unbalanced,
}

impl Display for Budget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Budget: {}\nTotal Income: ${:.2}\n", self.name, self.total_income)
    }
}

impl Budget {
    pub fn new(name: String, user_id: Uuid, total_income: f32, default_budget: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            user_id,
            total_income,
            budget_groups: HashMap::new(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            default_budget,
        }
    }

    /// Add a new budget group
    pub fn add_group(&mut self, group_name: String) -> &mut BudgetGroup {
        let group = BudgetGroup {
            id: Uuid::new_v4(),
            name: group_name.clone(),
            items: Vec::new(),
        };

        self.budget_groups.insert(group_name.clone(), group);
        self.touch();
        self.budget_groups.get_mut(&group_name).unwrap()
    }

    /// Add a budget item to a specific group
    pub fn add_item_to_group(&mut self, group_name: &str, item: BudgetItem) -> Result<(), String> {
        if let Some(group) = self.budget_groups.get_mut(group_name) {
            group.items.push(item);
            self.touch();
            Ok(())
        } else {
            Err(format!("Group '{}' not found", group_name))
        }
    }

    /// Create and add a new budget item
    pub fn create_budget_item(
        &mut self,
        group_name: &str,
        name: String,
        item_type: BudgetItemType,
        budgeted_amount: f32,
    ) -> Result<Uuid, String> {
        let item = BudgetItem {
            id: Uuid::new_v4(),
            name,
            item_type,
            budgeted_amount,
            actual_spent: 0.0,
            notes: None,
        };

        let item_id = item.id;
        self.add_item_to_group(group_name, item)?;
        Ok(item_id)
    }

    /// Calculate total amount budgeted across all items
    pub fn total_budgeted(&self) -> f32 {
        self.budget_groups
            .values()
            .flat_map(|group| &group.items)
            .map(|item| item.budgeted_amount)
            .sum()
    }

    /// Calculate total amount actually spent
    pub fn total_spent(&self) -> f32 {
        self.budget_groups
            .values()
            .flat_map(|group| &group.items)
            .map(|item| item.actual_spent)
            .sum()
    }

    /// Calculate unallocated income (income not yet budgeted)
    pub fn unallocated_income(&self) -> f32 {
        self.total_income - self.total_budgeted()
    }

    /// Check if budget is balanced (all income is allocated)
    pub fn is_balanced(&self) -> bool {
        (self.total_income - self.total_budgeted()).abs() < 0.01 // Allow for floating point precision
    }

    /// Record a bank transaction against a budget item
    pub fn record_transaction(&mut self, transaction: BankTransaction) -> Result<(), String> {
        if let Some(item_id) = transaction.budget_item_id {
            if let Some(item) = self.find_item_mut(item_id) {
                item.actual_spent += transaction.amount;
                self.touch();
                Ok(())
            } else {
                Err(format!("Budget item with ID {} not found", item_id))
            }
        } else {
            Err("Transaction must be associated with a budget item".to_string())
        }
    }

    /// Reallocate money between budget items (maintaining balance)
    pub fn reallocate_funds(
        &mut self,
        from_item_id: Uuid,
        to_item_id: Uuid,
        amount: f32,
    ) -> Result<(), String> {
        if amount <= 0.0 {
            return Err("Amount must be positive".to_string());
        }

        // Find both items
        let from_item = self.find_item(from_item_id)
            .ok_or("Source item not found")?;

        if from_item.budgeted_amount < amount {
            return Err("Insufficient funds in source item".to_string());
        }

        // Perform the reallocation
        let from_item = self.find_item_mut(from_item_id).unwrap();
        from_item.budgeted_amount -= amount;

        let to_item = self.find_item_mut(to_item_id).unwrap();
        to_item.budgeted_amount += amount;

        self.touch();
        Ok(())
    }

    /// Generate a comprehensive budget summary
    pub fn generate_summary(&self) -> BudgetSummary {
        let mut issues = Vec::new();

        // Check for overspent items
        for group in self.budget_groups.values() {
            for item in &group.items {
                if item.actual_spent > item.budgeted_amount {
                    let overspent_amount = item.actual_spent - item.budgeted_amount;
                    issues.push(BudgetIssue {
                        issue_type: BudgetIssueType::Overspent,
                        item_id: item.id,
                        item_name: item.name.clone(),
                        group_name: group.name.clone(),
                        amount: overspent_amount,
                        description: format!(
                            "Overspent by ${:.2} (spent ${:.2}, budgeted ${:.2})",
                            overspent_amount, item.actual_spent, item.budgeted_amount
                        ),
                    });
                }
            }
        }

        // Check if budget is unbalanced
        if !self.is_balanced() {
            let unallocated = self.unallocated_income();
            if unallocated.abs() > 0.01 {
                issues.push(BudgetIssue {
                    issue_type: BudgetIssueType::Unbalanced,
                    item_id: Uuid::nil(), // No specific item
                    item_name: "Budget Balance".to_string(),
                    group_name: "Overall".to_string(),
                    amount: unallocated.abs(),
                    description: if unallocated > 0.0 {
                        format!("${:.2} income not yet allocated to budget items", unallocated)
                    } else {
                        format!("Budget exceeds income by ${:.2}", unallocated.abs())
                    },
                });
            }
        }

        BudgetSummary {
            id: self.id,
            name: self.name.clone(),
            total_income: self.total_income,
            total_budgeted: self.total_budgeted(),
            total_spent: self.total_spent(),
            is_balanced: self.is_balanced(),
            unallocated_income: self.unallocated_income(),
            issues,
            default_budget: self.default_budget,
        }
    }

    /// Helper to find an item by ID
    fn find_item(&self, item_id: Uuid) -> Option<&BudgetItem> {
        self.budget_groups
            .values()
            .flat_map(|group| &group.items)
            .find(|item| item.id == item_id)
    }

    /// Helper to find a mutable item by ID
    fn find_item_mut(&mut self, item_id: Uuid) -> Option<&mut BudgetItem> {
        self.budget_groups
            .values_mut()
            .flat_map(|group| &mut group.items)
            .find(|item| item.id == item_id)
    }

    /// Update the timestamp
    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

impl BudgetItem {
    /// How much of the budgeted amount is remaining
    pub fn remaining_budget(&self) -> f32 {
        self.budgeted_amount - self.actual_spent
    }

    /// Percentage of budget used
    pub fn budget_utilization(&self) -> f32 {
        if self.budgeted_amount > 0.0 {
            (self.actual_spent / self.budgeted_amount * 100.0).min(100.0)
        } else {
            0.0
        }
    }

    /// Is this item overspent?
    pub fn is_overspent(&self) -> bool {
        self.actual_spent > self.budgeted_amount
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_budget() -> Budget {
        Budget::new("Test Budget".to_string(), Uuid::new_v4(), 5000.0, true)
    }

    #[test]
    fn test_budget_creation() {
        let budget = create_test_budget();
        assert_eq!(budget.name, "Test Budget");
        assert_eq!(budget.total_income, 5000.0);
        assert_eq!(budget.total_budgeted(), 0.0);
        assert!(budget.is_balanced()); // Empty budget is balanced
    }

    #[test]
    fn test_adding_groups_and_items() {
        let mut budget = create_test_budget();

        // Add a group
        budget.add_group("Household".to_string());

        // Add an item
        let item_id = budget.create_budget_item(
            "Household",
            "Rent".to_string(),
            BudgetItemType::Expense,
            1500.0,
        ).unwrap();

        assert_eq!(budget.total_budgeted(), 1500.0);
        assert_eq!(budget.unallocated_income(), 3500.0);
        assert!(!budget.is_balanced());
    }

    #[test]
    fn test_reallocation() {
        let mut budget = create_test_budget();
        budget.add_group("Household".to_string());

        let rent_id = budget.create_budget_item(
            "Household",
            "Rent".to_string(),
            BudgetItemType::Expense,
            1500.0,
        ).unwrap();

        let groceries_id = budget.create_budget_item(
            "Household",
            "Groceries".to_string(),
            BudgetItemType::Expense,
            500.0,
        ).unwrap();

        // Reallocate $200 from rent to groceries
        budget.reallocate_funds(rent_id, groceries_id, 200.0).unwrap();

        let rent = budget.find_item(rent_id).unwrap();
        let groceries = budget.find_item(groceries_id).unwrap();

        assert_eq!(rent.budgeted_amount, 1300.0);
        assert_eq!(groceries.budgeted_amount, 700.0);
        assert_eq!(budget.total_budgeted(), 2000.0); // Total unchanged
    }

    #[test]
    fn test_overspending_detection() {
        let mut budget = create_test_budget();
        budget.add_group("Household".to_string());

        let groceries_id = budget.create_budget_item(
            "Household",
            "Groceries".to_string(),
            BudgetItemType::Expense,
            500.0,
        ).unwrap();

        // Record overspending
        let transaction = BankTransaction {
            id: Uuid::new_v4(),
            amount: 600.0,
            description: "Grocery shopping".to_string(),
            date: chrono::Utc::now().naive_utc().date(),
            budget_item_id: Some(groceries_id),
        };

        budget.record_transaction(transaction).unwrap();

        let summary = budget.generate_summary();
        assert_eq!(summary.issues.len(), 2); // Overspent + Unbalanced

        let overspent_issue = summary.issues.iter()
            .find(|i| matches!(i.issue_type, BudgetIssueType::Overspent))
            .unwrap();
        assert_eq!(overspent_issue.amount, 100.0);
    }
}
