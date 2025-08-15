use joydb::Model;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::str::FromStr;
use serde::de::{MapAccess, Visitor};
use serde::ser::SerializeMap;
use uuid::Uuid;
use crate::models::budget_category::BudgetCategory;
use crate::models::budget_item::BudgetItem;
use crate::models::budget_transaction::BudgetTransaction;
use crate::models::budget_transaction_type::BudgetTransactionType;
use crate::models::month_begins_on::MonthBeginsOn;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default, Model)]
pub struct Budget {
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub month_begins_on: MonthBeginsOn,
    #[serde(serialize_with = "serialize_budget_items_as_string_keys",
        deserialize_with = "deserialize_budget_items_with_string_keys")]
    pub budget_items: HashMap<BudgetCategory, Vec<BudgetItem>>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub user_id: Uuid,
}

impl Display for Budget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "╭─ Budget: {} ─╮", self.name)?;
        writeln!(f, "│ ID: {}", self.id)?;
        writeln!(f, "│ Default Budget: {}", if self.default_budget { "Yes" } else { "No" })?;
        writeln!(f, "│ User ID: {}", self.user_id)?;
        
        // Format month begins on
        let month_info = match self.month_begins_on {
            MonthBeginsOn::PreviousMonth(day) => format!("Previous month, day {}", day),
            MonthBeginsOn::CurrentMonth(day) => format!("Current month, day {}", day),
        };
        writeln!(f, "│ Month Begins On: {}", month_info)?;
        
        writeln!(f, "│ Created: {}", self.created_at.format("%Y-%m-%d %H:%M:%S"))?;
        writeln!(f, "│ Updated: {}", self.updated_at.format("%Y-%m-%d %H:%M:%S"))?;
        
        // Calculate totals
        let total_income = self.budget_items.iter()
            .filter(|(category, _)| matches!(category, BudgetCategory::Income(_)))
            .flat_map(|(_, items)| items)
            .map(|item| item.budgeted_item_amount())
            .sum::<f32>();
            
        let total_expenses = self.budget_items.iter()
            .filter(|(category, _)| matches!(category, BudgetCategory::Expense(_)))
            .flat_map(|(_, items)| items)
            .map(|item| item.budgeted_item_amount())
            .sum::<f32>();
            
        let available_spendable = self.get_available_spendable_budget();
        
        writeln!(f, "│")?;
        writeln!(f, "│ 💰 Financial Summary:")?;
        writeln!(f, "│   Total Income:     ${:.2}", total_income)?;
        writeln!(f, "│   Total Expenses:   ${:.2}", total_expenses)?;
        writeln!(f, "│   Available:        ${:.2}", available_spendable)?;
        writeln!(f, "│   Balance:          ${:.2}", total_income - total_expenses)?;
        
        writeln!(f, "│")?;
        writeln!(f, "│ 📊 Budget Categories ({} total):", self.budget_items.len())?;
        
        // Sort categories for consistent display
        let mut sorted_categories: Vec<_> = self.budget_items.iter().collect();
        sorted_categories.sort_by(|a, b| {
            match (a.0, b.0) {
                (BudgetCategory::Income(_), BudgetCategory::Expense(_)) => std::cmp::Ordering::Less,
                (BudgetCategory::Expense(_), BudgetCategory::Income(_)) => std::cmp::Ordering::Greater,
                (BudgetCategory::Income(a), BudgetCategory::Income(b)) => a.cmp(b),
                (BudgetCategory::Expense(a), BudgetCategory::Expense(b)) => a.cmp(b),
                (_, _) => { std::cmp::Ordering::Less }
            }
        });
        
        for (category, items) in sorted_categories {
            let category_icon = match category {
                BudgetCategory::Income(_) => "💵",
                BudgetCategory::Expense(_) => "💸",
                BudgetCategory::Savings(_) => "💰"
            };
            
            let category_total: f32 = items.iter().map(|item| item.budgeted_item_amount()).sum();
            let category_spent: f32 = items.iter().map(|item| item.total_bank_amount()).sum();
            let category_remaining: f32 = category_total - category_spent;
            
            writeln!(f, "│")?;
            writeln!(f, "│   {} {} ({} items)", category_icon, category, items.len())?;
            writeln!(f, "│     Budgeted: ${:.2} | Spent: ${:.2} | Remaining: ${:.2}", 
                     category_total, category_spent, category_remaining)?;
            
            // Show individual budget items if there are any
            if !items.is_empty() {
                for (i, item) in items.iter().enumerate() {
                    let item_remaining = item.budgeted_item_amount();
                    let progress_bar = if item.budgeted_item_amount() > 0.0 {
                        let percentage = (item.total_bank_amount() / item.budgeted_item_amount() * 100.0).min(100.0);
                        let filled = (percentage / 10.0) as usize;
                        let empty = 10 - filled;
                        format!("[{}{}] {:.1}%", 
                               "█".repeat(filled), 
                               "░".repeat(empty), 
                               percentage)
                    } else {
                        "[──────────] N/A".to_string()
                    };
                    
                    let prefix = if i == items.len() - 1 { "└─" } else { "├─" };
                    writeln!(f, "│       {} {}", prefix, item.name)?;
                    writeln!(f, "│       {}   ${:.2}/${:.2} remaining | {}",
                             if i == items.len() - 1 { "   " } else { "│  " },
                             item_remaining, item.budgeted_item_amount(), progress_bar)?;
                }
            }
        }
        
        writeln!(f, "╰{}╯", "─".repeat(self.name.len() + 12))
    }
}

impl PartialEq for Budget {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Budget {
    pub fn get_available_spendable_budget(&self) -> f32 {
        self.budget_items
            .iter()
            .filter_map(|(key, items)| {
                if matches!(key, BudgetCategory::Income(_)) {
                    Some(
                        items.iter().map(|i| i.budgeted_item_amount()).sum::<f32>(),
                    )
                } else {
                    None
                }
            })
            .sum()
    }

    pub fn spend_money_on(&mut self, target: &mut BudgetItem, amount: f32) {
        if amount > 0.0 && amount <= self.get_available_spendable_budget() {
            /* 
            Some splitting logic needed here, we need to split the amount
            over multiple budget items if not one can fit the entire amount
             */
            let mut amount_left = amount;
            for (category, items) in &mut self.budget_items {
                if matches!(category, BudgetCategory::Income(_)) {
                    for item in items.iter_mut().filter(|i| i.budgeted_item_amount() > 0.0) {
                        if item.budgeted_item_amount() > amount_left {
                            let transaction = BudgetTransaction::new(
                                "Spend Money",
                                BudgetTransactionType::default(),
                                amount_left,
                                Some(item.id),
                                target.id,
                                self.user_id,
                            );
                            item.store_outgoing_transaction(&transaction);
                            target.store_incoming_transaction(&transaction);
                            amount_left = 0.0;
                            break;
                        } else {
                            let amount_to_spend = item.budgeted_item_amount();
                            let transaction = BudgetTransaction::new(
                                "Spend Money",
                                BudgetTransactionType::default(),
                                amount_to_spend,
                                Some(item.id),
                                target.id,
                                self.user_id,
                            );
                            item.store_outgoing_transaction(&transaction);
                            target.store_incoming_transaction(&transaction);
                            amount_left -= amount_to_spend;
                        }                        
                    }

                }
            }
        }
    }

    pub fn new(name: &str, default_budget: bool, user_id: Uuid) -> Budget {
        Budget {
            id: Uuid::new_v4(),
            name: name.to_string(),
            default_budget,
            month_begins_on: MonthBeginsOn::PreviousMonth(25),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            user_id,
            ..Default::default()
        }
    }

    pub fn new_income_category(&mut self, category_name: &str) -> BudgetCategory {
        let category = BudgetCategory::Income(category_name.to_string());
        self.store_category(&category);
        category
    }
    
    pub fn store_category(&mut self, category: &BudgetCategory) {
        if let Vacant(e) = self.budget_items.entry(category.clone()) {
            e.insert(Vec::new());
        }
        self.touch();
    }

    pub fn new_expense_category(&mut self, category_name: &str) -> BudgetCategory {
        let category = BudgetCategory::Expense(category_name.to_string());
        self.store_category(&category);
        category

    }

    pub fn store_budget_item(&mut self, budget_item: &BudgetItem) {
        match self.budget_items.entry(budget_item.budget_category.clone()) {
            Vacant(e) => {
                e.insert(vec![budget_item.clone()]);
            }
            Occupied(mut e) => {
                e.get_mut().push(budget_item.clone());
            }
        }

        self.touch();
    }

    /// Generates an overview of budget items that require user attention or action.
    ///
    /// Returns:
    /// - Income items that are not balanced (have unallocated money)
    /// - Expense items that are overdrawn (bank spending exceeds budgeted amount)
    pub fn generate_actionable_overview(&self) -> BudgetActionOverview {
        let mut action_items = Vec::new();
        let mut total_unallocated_income = 0.0;
        let mut total_overdrawn_amount = 0.0;

        for (category, items) in &self.budget_items {
            for item in items {
                match category {
                    BudgetCategory::Income(_) => {
                        // Income items with unallocated money (not all money has outgoing transactions)
                        let unallocated = item.budgeted_item_amount();
                        if unallocated > 0.0 {
                            action_items.push(BudgetActionItem {
                                id: item.id,
                                name: item.name.clone(),
                                category: category.clone(),
                                issue_type: ActionItemType::UnallocatedIncome,
                                amount: unallocated,
                                description: format!(
                                    "Has ${:.2} unallocated income that needs to be budgeted",
                                    unallocated
                                ),
                            });
                            total_unallocated_income += unallocated;
                        }
                    }
                    BudgetCategory::Expense(_) => {
                        // Expense items where bank spending exceeds budgeted amount
                        let bank_spending = item.total_bank_amount().abs(); // Use abs since bank transactions are typically negative
                        let budgeted_amount = item.budgeted_item_amount();

                        if bank_spending > budgeted_amount {
                            let overdrawn_amount = bank_spending - budgeted_amount;
                            action_items.push(BudgetActionItem {
                                id: item.id,
                                name: item.name.clone(),
                                category: category.clone(),
                                issue_type: ActionItemType::OverdrawnExpense,
                                amount: overdrawn_amount,
                                description: format!(
                                    "Overdrawn by ${:.2} (spent ${:.2}, budgeted ${:.2})",
                                    overdrawn_amount, bank_spending, budgeted_amount
                                ),
                            });
                            total_overdrawn_amount += overdrawn_amount;
                        }
                    }
                    BudgetCategory::Savings(_) => {
                        // Savings items could be handled similarly to expenses if needed
                        // For now, we'll skip them as they weren't mentioned in requirements
                    }
                }
            }
        }

        BudgetActionOverview {
            budget_id: self.id,
            budget_name: self.name.clone(),
            action_items,
            total_unallocated_income,
            total_overdrawn_amount,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().naive_utc();
    }
}

/// Represents an item that requires user attention or action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetActionItem {
    pub id: Uuid,
    pub name: String,
    pub category: BudgetCategory,
    pub issue_type: ActionItemType,
    pub amount: f32,
    pub description: String,
}

/// Types of issues that require user action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActionItemType {
    /// Income item has unallocated money (not balanced)
    UnallocatedIncome,
    /// Expense item is overdrawn (bank spending exceeds budget)
    OverdrawnExpense,
}

/// Overview of items requiring user attention
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetActionOverview {
    pub budget_id: Uuid,
    pub budget_name: String,
    pub action_items: Vec<BudgetActionItem>,
    pub total_unallocated_income: f32,
    pub total_overdrawn_amount: f32,
}


fn deserialize_budget_items_with_string_keys<'de, D>(
    deserializer: D,
) -> Result<HashMap<BudgetCategory, Vec<BudgetItem>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct BudgetMapVisitor {
        marker: PhantomData<fn() -> HashMap<BudgetCategory, Vec<BudgetItem>>>,
    }

    impl<'de> Visitor<'de> for BudgetMapVisitor {
        type Value = HashMap<BudgetCategory, Vec<BudgetItem>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map with stringified BudgetCategory keys")
        }

        fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
        where
            M: MapAccess<'de>,
        {
            let mut map = HashMap::new();
            while let Some((key_str, value)) = access.next_entry::<String, Vec<BudgetItem>>()? {
                let key = BudgetCategory::from_str(&key_str)
                    .map_err(|e| de::Error::custom(format!("Key parse error: {}", e)))?;
                map.insert(key, value);
            }
            Ok(map)
        }
    }

    deserializer.deserialize_map(BudgetMapVisitor {
        marker: PhantomData,
    })
}

fn serialize_budget_items_as_string_keys<S>(
    map: &HashMap<BudgetCategory, Vec<BudgetItem>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map_ser = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        map_ser.serialize_entry(&k.to_string(), v)?;
    }
    map_ser.end()
}

