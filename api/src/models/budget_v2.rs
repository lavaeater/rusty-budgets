use joydb::Model;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use uuid::Uuid;

/// A simplified, more intuitive budget model
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub user_id: Uuid,

    /// Organized budget items by group
    pub budget_groups: HashMap<String, BudgetGroup>,

    pub bank_transactions: Vec<BankTransaction>,

    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
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
    pub tags: Vec<String>,
}

impl PartialEq for BudgetItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.item_type == other.item_type
            && self.budgeted_amount == other.budgeted_amount
            && self.actual_spent == other.actual_spent
            && (match &self.notes {
            None => other.notes.is_none(),
            Some(self_id) => match &other.notes {
                None => false,
                Some(other_id) => self_id == other_id,
            },
        })
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetItemType {
    Income,
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

impl BankTransaction {
    pub fn new(amount: f32, description: String, date: chrono::NaiveDate) -> Self {
        BankTransaction {
            id: Uuid::new_v4(),
            amount,
            description,
            date,
            budget_item_id: None,
        }
    }
}

impl PartialEq for BankTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.amount == other.amount
            && self.description == other.description
            && self.date == other.date
            && (match self.budget_item_id {
            None => match other.budget_item_id {
                None => true,
                _ => false,
            },
            Some(self_id) => match other.budget_item_id {
                None => false,
                Some(other_id) => self_id == other_id,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetItemSummary {
    pub id: Uuid,
    pub name: String,
    pub item_type: BudgetItemType,
    pub budgeted_amount: f32,
    pub left_to_spend: f32,
    pub tags: Vec<String>,
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
    pub item_summaries: Vec<BudgetItemSummary>,
    pub default_budget: bool,
}


/// Issues that require user attention
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetIssue {
    pub issue_type: BudgetIssueType,
    pub amount: f32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BudgetIssueType {
    /// Item has been overspent (actual > budgeted)
    Overspent(BudgetItem),
    /// Budget is not balanced (total budgeted != total income)
    Unbalanced,
    TransactionNotConnected(BankTransaction),
}

impl Display for BudgetIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetIssueType::Overspent(item) => write!(f, "Overspent: {}", item.name),
            BudgetIssueType::Unbalanced => write!(f, "Unbalanced budget"),
            BudgetIssueType::TransactionNotConnected(transaction) => {
                write!(f, "Transaction not connected: {}", transaction.description)
            }
        }
    }
}

impl Display for Budget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Budget: {}\nTotal Income: ${:.2}\n",
            self.name,
            self.total_income()
        )
    }
}

impl Budget {
    pub fn new(name: String, user_id: Uuid, default_budget: bool) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            user_id,
            budget_groups: HashMap::new(),
            bank_transactions: Vec::new(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            default_budget,
        }
    }

    pub fn add_bank_transaction(
        &mut self,
        amount: f32,
        description: &str,
        date: chrono::NaiveDate,
    ) {
        let transaction = BankTransaction::new(amount, description.to_string(), date);
        self.bank_transactions.push(transaction);
        self.touch();
    }

    pub fn get_group(&self, group_name: &str) -> Option<&BudgetGroup> {
        self.budget_groups.get(group_name)
    }

    pub fn non_handled_bank_transactions(&self) -> Vec<BankTransaction> {
        self.bank_transactions
            .iter()
            .filter(|t| t.budget_item_id.is_none())
            .cloned()
            .collect()
    }

    pub fn find_unhandled_budget_item_transactions(&self, budget_item_id: Uuid) -> Vec<BankTransaction> {
        self.bank_transactions
            .iter()
            .filter(|t| t.budget_item_id == Some(budget_item_id))
            .cloned()
            .collect()
    }

    pub fn connect_bank_transaction_with_item(&mut self, transaction_id: Uuid, item_id: Uuid) {
        if let Some(transaction) = self
            .bank_transactions
            .iter_mut()
            .find(|t| t.id == transaction_id)
        {
            transaction.budget_item_id = Some(item_id);
            let _ = self.update_item_actual_spent(item_id);
            self.touch();
        }
    }

    pub fn update_item_actual_spent(&mut self, item_id: Uuid) -> Result<(), String> {
        if let Some(item) = self.budget_groups
            .values_mut()
            .flat_map(|group| &mut group.items)
            .find(|item| item.id == item_id) {
            item.actual_spent = self.bank_transactions.iter().filter(|t| t.budget_item_id == Some(item_id)).fold(0.0, |acc, t| acc + t.amount);

            // transaction.budget_item_id {
            //     if let Some(item) = self.find_item_mut(item_id) {
            //         item.actual_spent += transaction.amount;
            //         self.touch();
            //         Ok(())
            //     } else {
            //         Err(format!("Budget item with ID {} not found", item_id))
            //     }
            // } else {
            //     Err("Transaction must be associated with a budget item".to_string())
            // }
        }
        Ok(())
    }


    /// Add a new budget group
        pub fn add_group(&mut self, group_name: &str) -> &mut BudgetGroup {
            let group = BudgetGroup {
                id: Uuid::new_v4(),
                name: group_name.to_string(),
                items: Vec::new(),
            };

            self.budget_groups.insert(group_name.to_string(), group);
            self.touch();
            self.budget_groups.get_mut(group_name).unwrap()
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
            name: &str,
            item_type: BudgetItemType,
            budgeted_amount: f32,
        ) -> Result<Uuid, String> {
            let item = BudgetItem {
                id: Uuid::new_v4(),
                name: name.to_string(),
                item_type,
                budgeted_amount,
                actual_spent: 0.0,
                notes: None,
                tags: Vec::new(),
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
                .filter(|item| item.item_type != BudgetItemType::Income)
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

        pub fn total_income(&self) -> f32 {
            self.budget_groups
                .values()
                .flat_map(|group| &group.items)
                .filter(|item| item.item_type == BudgetItemType::Income)
                .map(|item| item.budgeted_amount)
                .sum()
        }

        /// Calculate unallocated income (income not yet budgeted)
        pub fn unallocated_income(&self) -> f32 {
            self.total_income() - self.total_budgeted()
        }

        /// Check if budget is balanced (all income is allocated)
        pub fn is_balanced(&self) -> bool {
            (self.total_income() - self.total_budgeted()).abs() < 0.01 // Allow for floating point precision
        }

        /// Record a bank transaction against a budget item

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
            let from_item = self
                .find_item(from_item_id)
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
            let mut item_summaries = self
                .budget_groups
                .values()
                .flat_map(|group| {
                    group.items.iter()
                        .filter(|item| item.item_type == BudgetItemType::Expense)
                        .map(|item| BudgetItemSummary {
                        id: item.id,
                        name: item.name.clone(),
                        item_type: item.item_type,
                        budgeted_amount: item.budgeted_amount,
                        left_to_spend: item.budgeted_amount + item.actual_spent,
                        tags: item.tags.clone(),
                    })
                })
                .collect::<Vec<BudgetItemSummary>>();
            
            item_summaries.sort_by(|a, b| {b.left_to_spend.partial_cmp(&a.left_to_spend).unwrap()});

            // Check for overspent items
            for group in self.budget_groups.values() {
                for item in &group.items {
                    if item.actual_spent > item.budgeted_amount {
                        let overspent_amount = item.actual_spent - item.budgeted_amount;
                        issues.push(BudgetIssue {
                            issue_type: BudgetIssueType::Overspent(item.clone()),
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
                        amount: unallocated.abs(),
                        description: if unallocated > 0.0 {
                            format!(
                                "${:.2} income not yet allocated to budget items",
                                unallocated
                            )
                        } else {
                            format!("Budget exceeds income by ${:.2}", unallocated.abs())
                        },
                    });
                }
            }
            for transaction in &self.non_handled_bank_transactions() {
                issues.push(BudgetIssue {
                    issue_type: BudgetIssueType::TransactionNotConnected(transaction.clone()),
                    amount: transaction.amount,
                    description: transaction.description.clone(),
                });
            }

            BudgetSummary {
                id: self.id,
                name: self.name.clone(),
                total_income: self.total_income(),
                total_budgeted: self.total_budgeted(),
                total_spent: self.total_spent(),
                is_balanced: self.is_balanced(),
                unallocated_income: self.unallocated_income(),
                issues,
                default_budget: self.default_budget,
                item_summaries,
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

        /// Create a test budget with a single income item
        ///
        /// # Returns
        ///
        /// A new `Budget` instance with a single income item named "Tommie's Lön" in the "Löner" group.
        /// The budget is named "Test Budget", belongs to a new UUID, and has a default budget of 5000.0.
        fn create_test_budget(salary_amount: Option<f32>) -> Budget {
            let mut b = Budget::new("Test Budget".to_string(), Uuid::new_v4(), true);
            b.add_group("Löner");
            b.create_budget_item(
                "Löner",
                "Tommie's Lön",
                BudgetItemType::Income,
                if let Some(amount) = salary_amount {
                    amount
                } else {
                    5000.0
                },
            )
                .unwrap();
            b
        }

        #[test]
        fn test_budget_creation() {
            let mut budget = create_test_budget(None);

            let group_name = "Löner";

            budget.add_group(group_name);
            budget
                .create_budget_item(group_name, "Tommie's Lön", BudgetItemType::Income, 5000.0)
                .unwrap();
            assert_eq!(budget.name, "Test Budget");
            assert_eq!(budget.total_income(), 5000.0);
            assert_eq!(budget.total_budgeted(), 0.0);
            assert!(!budget.is_balanced()); // Empty budget is balanced
        }

        #[test]
        fn test_connecting_bank_transactions() {
            let mut budget = create_test_budget(Some(35000.0));
            budget.add_group("Household");
            let groceries_item_id = budget
                .create_budget_item("Household", "Groceries", BudgetItemType::Expense, 1500.0)
                .unwrap();
            let utilities_item_id = budget
                .create_budget_item("Household", "Utilities", BudgetItemType::Expense, 1500.0)
                .unwrap();
            let salary_group_id = budget.get_group("Löner").unwrap().id;

            budget.add_bank_transaction(
                -1500.0,
                "Groceries",
                chrono::NaiveDate::from_ymd_opt(2023, 6, 1).unwrap(),
            );
            budget.add_bank_transaction(
                35000.0,
                "Snakes",
                chrono::NaiveDate::from_ymd_opt(2023, 6, 2).unwrap(),
            );
            budget.add_bank_transaction(
                -2500.0,
                "Utilities",
                chrono::NaiveDate::from_ymd_opt(2023, 6, 3).unwrap(),
            );

            // Collect transaction info first to avoid borrowing conflicts
            let transaction_connections = budget.non_handled_bank_transactions();

            for transaction in transaction_connections {
                match transaction.description.as_str() {
                    "Groceries" => {
                        budget.connect_bank_transaction_with_item(transaction.id, groceries_item_id)
                    }
                    "Snakes" => {
                        budget.connect_bank_transaction_with_item(transaction.id, salary_group_id)
                    }
                    "Utilities" => {
                        budget.connect_bank_transaction_with_item(transaction.id, utilities_item_id)
                    }
                    _ => panic!(
                        "Unexpected transaction description: {}",
                        transaction.description
                    ),
                }
            }
            
            let summary = budget.generate_summary();
            println!("Summary: {:#?}", summary);
        }

        #[test]
        fn test_adding_groups_and_items() {
            let mut budget = create_test_budget(None);

            // Add a group
            budget.add_group("Household");

            // Add an item
            let _ = budget
                .create_budget_item("Household", "Rent", BudgetItemType::Expense, 1500.0)
                .unwrap();

            assert_eq!(budget.total_budgeted(), 1500.0);
            assert_eq!(budget.unallocated_income(), 3500.0);
            assert!(!budget.is_balanced());
        }

        #[test]
        fn test_reallocation() {
            let mut budget = create_test_budget(None);
            budget.add_group("Household");

            let rent_id = budget
                .create_budget_item("Household", "Rent", BudgetItemType::Expense, 1500.0)
                .unwrap();

            let groceries_id = budget
                .create_budget_item("Household", "Groceries", BudgetItemType::Expense, 500.0)
                .unwrap();

            // Reallocate $200 from rent to groceries
            budget
                .reallocate_funds(rent_id, groceries_id, 200.0)
                .unwrap();

            let rent = budget.find_item(rent_id).unwrap();
            let groceries = budget.find_item(groceries_id).unwrap();

            assert_eq!(rent.budgeted_amount, 1300.0);
            assert_eq!(groceries.budgeted_amount, 700.0);
            assert_eq!(budget.total_budgeted(), 2000.0); // Total unchanged
        }

        // #[test]
        // fn test_overspending_detection() {
        //     let mut budget = create_test_budget(None);
        //     budget.add_group("Household");
        // 
        //     let groceries_id = budget
        //         .create_budget_item("Household", "Groceries", BudgetItemType::Expense, 500.0)
        //         .unwrap();
        // 
        //     // Record overspending
        //     let transaction = BankTransaction {
        //         id: Uuid::new_v4(),
        //         amount: 600.0,
        //         description: "Grocery shopping".to_string(),
        //         date: chrono::Utc::now().naive_utc().date(),
        //         budget_item_id: Some(groceries_id),
        //     };
        // 
        //     budget.record_transaction(transaction).unwrap();
        // 
        //     let summary = budget.generate_summary();
        //     assert_eq!(summary.issues.len(), 2); // Overspent + Unbalanced
        // 
        //     let overspent_issue = summary
        //         .issues
        //         .iter()
        //         .find(|i| matches!(i.issue_type, BudgetIssueType::Overspent))
        //         .unwrap();
        //     assert_eq!(overspent_issue.amount, 100.0);
        // }
    }
