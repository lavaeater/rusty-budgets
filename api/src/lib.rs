//! This crate contains all shared fullstack server functions.
pub mod models;

use crate::models::*;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use joydb::Joydb;
#[cfg(feature = "server")]
use dioxus::logger::tracing;
#[cfg(feature = "server")]
use joydb::adapters::JsonAdapter;

#[cfg(feature = "server")]
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";
// Define the state
joydb::state! {
    AppState,
    models: [User, Budget, BudgetItem, BankTransaction],
}

// Define the database (combination of state and adapter)
#[cfg(feature = "server")]
type Db = Joydb<AppState, JsonAdapter>;
#[cfg(feature = "server")]
pub mod db {
    use crate::models::*;
    use crate::{Db, DEFAULT_USER_EMAIL};
    use chrono::NaiveDate;
    use dioxus::fullstack::once_cell::sync::Lazy;
    use dioxus::logger::tracing;
    use uuid::Uuid;

    pub static CLIENT: Lazy<Db> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = Db::open("./data.json").unwrap();
        // Run migrations
        tracing::info!("Insert Default Data");
        match get_default_user(Some(&client)) {
            Ok(user) => {
                tracing::info!("Default user exists");
                match get_default_budget_for_user(user.id, Some(&client)) {
                    Ok(budget) => {
                        tracing::info!("Default budget exists: {}", budget);
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Could not get default budget for user");
                        panic!("Could not get default budget for user");
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                panic!("Could not get default user");
            }
        }
        client
    });

    fn client_from_option(client: Option<&Db>) -> &Db {
        if let Some(c) = client {
            c
        } else {
            &CLIENT
        }
    }
    
    pub fn list_users(client: Option<&Db>) -> anyhow::Result<Vec<User>> {
        match client_from_option(client).get_all::<User>() {
            Ok(users) => Ok(users),
            Err(e) => {
                tracing::error!(error = %e, "Could not list users");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn user_exists(email: &str, client: Option<&Db>) -> anyhow::Result<bool> {
        match client_from_option(client).get_all_by(|u: &User| u.email == email) {
            Ok(users) => Ok(!users.is_empty()),
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_user(client: Option<&Db>) -> anyhow::Result<User> {
        match client_from_option(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
            Ok(mut users) => {
                if users.is_empty() {
                    create_user(
                        "tommie",
                        DEFAULT_USER_EMAIL,
                        "Tommie",
                        "Nygren",
                        Some("0704382781".to_string()),
                        Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                        client,
                    )
                } else {
                    Ok(users.remove(0))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn save_budget(budget: Budget) -> anyhow::Result<()> {
        match client_from_option(None).update(&budget) {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget");
                Err(anyhow::Error::from(e))
            }
        }
    }
    
    pub fn get_default_budget_for_user(
        user_id: Uuid,
        client: Option<&Db>,
    ) -> anyhow::Result<Budget> {
        match client_from_option(client)
            .get_all_by(|b: &Budget| b.user_id == user_id && b.default_budget)
        {
            Ok(budgets) => {
                if budgets.is_empty() {
                    tracing::info!("No default budget exists, time to create one");
                    create_test_budget(user_id, client)
                } else {
                    let _ = client_from_option(client)
                        .delete_all_by(|b: &Budget| b.user_id == user_id && b.default_budget);
                    create_test_budget(user_id, client)
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default budget for user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_test_budget(user_id: Uuid, client: Option<&Db>) -> anyhow::Result<Budget> { 
        let mut budget = Budget::new("Test Budget".to_string(), user_id, true);
        
        // Create Income group with salary
        budget.add_group("Income");
        let salary_id = budget.create_budget_item(
            "Income", 
            "Monthly Salary", 
            BudgetItemType::Income, 
            45000.0
        ).unwrap();
        
        // Create Household group with essential expenses
        budget.add_group("Household");
        let rent_id = budget.create_budget_item(
            "Household", 
            "Rent", 
            BudgetItemType::Expense, 
            15000.0
        ).unwrap();
        
        let utilities_id = budget.create_budget_item(
            "Household", 
            "Utilities", 
            BudgetItemType::Expense, 
            2500.0
        ).unwrap();
        
        let groceries_id = budget.create_budget_item(
            "Household", 
            "Groceries", 
            BudgetItemType::Expense, 
            4000.0
        ).unwrap();
        
        let internet_id = budget.create_budget_item(
            "Household", 
            "Internet", 
            BudgetItemType::Expense, 
            500.0
        ).unwrap();
        
        // Create Pleasure group with entertainment expenses
        budget.add_group("Pleasure");
        let restaurants_id = budget.create_budget_item(
            "Pleasure", 
            "Restaurants", 
            BudgetItemType::Expense, 
            3000.0
        ).unwrap();
        
        let entertainment_id = budget.create_budget_item(
            "Pleasure", 
            "Entertainment", 
            BudgetItemType::Expense, 
            2000.0
        ).unwrap();
        
        let hobbies_id = budget.create_budget_item(
            "Pleasure", 
            "Hobbies", 
            BudgetItemType::Expense, 
            1500.0
        ).unwrap();
        
        // Create Savings group
        budget.add_group("Savings");
        let emergency_fund_id = budget.create_budget_item(
            "Savings", 
            "Emergency Fund", 
            BudgetItemType::Savings, 
            5000.0
        ).unwrap();
        
        let investments_id = budget.create_budget_item(
            "Savings", 
            "Investments", 
            BudgetItemType::Savings, 
            8000.0
        ).unwrap();
        
        // Add realistic bank transactions
        use chrono::NaiveDate;
        
        // Income transactions
        budget.add_bank_transaction(45000.0, "Monthly Salary - Company ABC", 
            NaiveDate::from_ymd_opt(2024, 8, 1).unwrap());
        
        // Household transactions
        budget.add_bank_transaction(-15000.0, "Rent Payment - Landlord", 
            NaiveDate::from_ymd_opt(2024, 8, 1).unwrap());
        budget.add_bank_transaction(-1200.0, "Electricity Bill", 
            NaiveDate::from_ymd_opt(2024, 8, 3).unwrap());
        budget.add_bank_transaction(-800.0, "Water Bill", 
            NaiveDate::from_ymd_opt(2024, 8, 5).unwrap());
        budget.add_bank_transaction(-500.0, "Gas Bill", 
            NaiveDate::from_ymd_opt(2024, 8, 7).unwrap());
        budget.add_bank_transaction(-500.0, "Internet Bill - ISP", 
            NaiveDate::from_ymd_opt(2024, 8, 10).unwrap());
        budget.add_bank_transaction(-1500.0, "ICA Supermarket", 
            NaiveDate::from_ymd_opt(2024, 8, 2).unwrap());
        budget.add_bank_transaction(-800.0, "Coop Grocery Store", 
            NaiveDate::from_ymd_opt(2024, 8, 8).unwrap());
        budget.add_bank_transaction(-1200.0, "Willys Supermarket", 
            NaiveDate::from_ymd_opt(2024, 8, 15).unwrap());
        budget.add_bank_transaction(-500.0, "Local Market", 
            NaiveDate::from_ymd_opt(2024, 8, 12).unwrap());
        
        // Pleasure transactions
        budget.add_bank_transaction(-850.0, "Restaurant Frantzén", 
            NaiveDate::from_ymd_opt(2024, 8, 4).unwrap());
        budget.add_bank_transaction(-450.0, "Café Saturnus", 
            NaiveDate::from_ymd_opt(2024, 8, 6).unwrap());
        budget.add_bank_transaction(-650.0, "Restaurant Oaxen", 
            NaiveDate::from_ymd_opt(2024, 8, 11).unwrap());
        budget.add_bank_transaction(-320.0, "Pizza Place", 
            NaiveDate::from_ymd_opt(2024, 8, 13).unwrap());
        budget.add_bank_transaction(-730.0, "Fine Dining", 
            NaiveDate::from_ymd_opt(2024, 8, 14).unwrap());
        budget.add_bank_transaction(-300.0, "Cinema Tickets", 
            NaiveDate::from_ymd_opt(2024, 8, 9).unwrap());
        budget.add_bank_transaction(-150.0, "Spotify Premium", 
            NaiveDate::from_ymd_opt(2024, 8, 1).unwrap());
        budget.add_bank_transaction(-1200.0, "Concert Tickets", 
            NaiveDate::from_ymd_opt(2024, 8, 16).unwrap());
        budget.add_bank_transaction(-350.0, "Netflix & Streaming", 
            NaiveDate::from_ymd_opt(2024, 8, 1).unwrap());
        budget.add_bank_transaction(-800.0, "Photography Equipment", 
            NaiveDate::from_ymd_opt(2024, 8, 7).unwrap());
        budget.add_bank_transaction(-700.0, "Sports Equipment", 
            NaiveDate::from_ymd_opt(2024, 8, 10).unwrap());
        
        // Savings transactions
        budget.add_bank_transaction(-5000.0, "Transfer to Emergency Fund", 
            NaiveDate::from_ymd_opt(2024, 8, 2).unwrap());
        budget.add_bank_transaction(-8000.0, "Investment Account Transfer", 
            NaiveDate::from_ymd_opt(2024, 8, 3).unwrap());
        
        // Connect transactions to budget items
        let transactions = budget.bank_transactions.clone();
        for transaction in transactions {
            let item_id = match transaction.description.as_str() {
                desc if desc.contains("Salary") => Some(salary_id),
                desc if desc.contains("Rent") => Some(rent_id),
                desc if desc.contains("Electricity") || desc.contains("Water") || desc.contains("Gas") => Some(utilities_id),
                desc if desc.contains("Internet") => Some(internet_id),
                desc if desc.contains("ICA") || desc.contains("Coop") || desc.contains("Willys") || desc.contains("Market") => Some(groceries_id),
                desc if desc.contains("Restaurant") || desc.contains("Café") || desc.contains("Pizza") || desc.contains("Fine Dining") => Some(restaurants_id),
                desc if desc.contains("Cinema") || desc.contains("Spotify") || desc.contains("Concert") || desc.contains("Netflix") => Some(entertainment_id),
                desc if desc.contains("Photography") || desc.contains("Sports") => Some(hobbies_id),
                desc if desc.contains("Emergency Fund") => Some(emergency_fund_id),
                desc if desc.contains("Investment") => Some(investments_id),
                _ => None,
            };
            
            if let Some(id) = item_id {
                budget.connect_bank_transaction_with_item(transaction.id, id);
            }
        }
        
        match serde_json::to_string(&budget) {
            Ok(b) => {
                tracing::info!(budget = %b, "Created test budget with realistic data");
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not serialize test budget");
            }
        }

        match client_from_option(client).insert(&budget) {
            Ok(_) => Ok(budget.clone()),
            Err(e) => {
                tracing::error!(error = %e, "Could not create test budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_budget(
        name: &str,
        user_id: Uuid,
        default_budget: bool,
        client: Option<&Db>,
    ) -> anyhow::Result<Budget> {
        let budget = Budget::new(name.to_string(), user_id, default_budget);
        match client_from_option(client).insert(&budget) {
            Ok(_) => Ok(budget.clone()),
            Err(e) => {
                tracing::error!(error = %e, "Could not create budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_user(
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
        client: Option<&Db>,
    ) -> anyhow::Result<User> {
        let user = User::new(user_name, email, first_name, last_name, phone, birthday);
        match client_from_option(client).insert(&user) {
            Ok(_) => Ok(user),
            Err(e) => {
                tracing::error!(error = %e, "Could not create user");
                Err(anyhow::Error::from(e))
            }
        }
    }
}

/// Echo the user input on the server.
#[server]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    match db::list_users(None) {
        Ok(users) => Ok(users),
        Err(e) => {
            tracing::error!(error = %e, "Could not list users");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget() -> Result<Budget, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).unwrap().id, None) {
        Ok(budget) => Ok(budget),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget_overview() -> Result<BudgetSummary, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).unwrap().id, None) {
        Ok(budget) => Ok(budget.generate_summary()),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn save_budget(budget: Budget) -> Result<(), ServerFnError> {
    match db::save_budget(budget) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
