//! This crate contains all shared fullstack server functions.
pub mod models;

use crate::models::*;
use chrono::NaiveDate;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    models: [User, Budget, BudgetItem, BudgetTransaction, BankTransaction],
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
    use joydb::JoydbError;
    use uuid::Uuid;
    use Default;

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

    pub fn add_budget_transaction(
        text: &str,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        amount: f32,
    ) -> anyhow::Result<()> {
        let user = get_default_user(None)?;
        let budget_transaction_to_save = BudgetTransaction::new(
            text,
            BudgetTransactionType::default(),
            amount,
            from_budget_item,
            to_budget_item,
            user.id,
        );
        match client_from_option(None).insert(&budget_transaction_to_save) {
            Ok(_) => {
                tracing::info!("Saved budget transaction");
                Ok(())
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget transaction");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn add_budget_item(
        budget_id: Uuid,
        name: String,
        first_item: &str,
        amount: f32
    ) -> anyhow::Result<()> {
        let user = get_default_user(None)?;
        let budget_item_to_save = BudgetItem::new(
            budget_id,
            &name,
            &BudgetCategory::Expense(name.clone()),
            user.id,
        );
        match client_from_option(None).insert(&budget_item_to_save) {
            Ok(_) => {
                tracing::info!("Saved budget item");
                add_budget_transaction(first_item, None, budget_item_to_save.id, amount)
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget item");
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
            Ok(mut budgets) => {
                if budgets.is_empty() {
                    tracing::info!("No default budget exists, time to create one");
                    create_test_budget(user_id, client)
                } else {
                    Ok(budgets.remove(0))
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default budget for user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn create_test_budget(user_id: Uuid, client: Option<&Db>) -> anyhow::Result<Budget> {
        let mut budget = Budget::new("Default test budget", true, user_id);
        //Create budget items
        let category = budget.new_income_category("Löner");
        //Incomes
        let mut salary_one = BudgetItem::new(
            budget.id,
            "Lön Tommie",
            &category,
            user_id,
        );
        
        let bt = BudgetTransaction::new(
            "Lön Tommie",
            BudgetTransactionType::default(),
            39500.0,
            None,
            salary_one.id,
            user_id,
        );
        
        salary_one.store_incoming_transaction(&bt);
        budget.store_budget_item(&salary_one);
        
        let mut salary_two = BudgetItem::new(
            budget.id,
            "Lön Lisa",
            &category,
            user_id,
        );
        
        let bt = BudgetTransaction::new(
            "Lön Lisa",
            BudgetTransactionType::default(),
            19500.0,
            None,
            salary_two.id,
            user_id,
        );
        salary_two.store_incoming_transaction(&bt);
        budget.store_budget_item(&salary_two);
        
        let category = budget.new_expense_category("Fasta Utgifter");
        
        let mut mortgage = BudgetItem::new(
            budget.id,
            "Huslånet",
            &category,
            user_id,
        );
        
        budget.spend_money_on(&mut mortgage, 5660.0);
        budget.store_budget_item(&mortgage);

        let mut utgift = BudgetItem::new(
            budget.id,
            "Hyra lägenheten",
            &category,
            user_id,
        );

        budget.spend_money_on(&mut utgift, 7500.0);
        budget.store_budget_item(&utgift);
        //Expenses
        
        match serde_json::to_string(&budget) {
            Ok(b) => {
                tracing::info!(budget = %b, "Created test budget");
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not serialize test budget");
            }
        }

        //Savings

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
        let budget = Budget::new(name, default_budget, user_id);
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
pub async fn get_default_budget_overview() -> Result<BudgetActionOverview, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).unwrap().id, None) {
        Ok(budget) => Ok(budget.generate_actionable_overview()),
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

#[server]
pub async fn add_budget_item(
    budget_id: Uuid,
    name: String,
    first_item: String,
    amount: f32,
) -> Result<(), ServerFnError> {
    tracing::info!(
        "add_budget_item: {}, {}, {}, {}",
        budget_id,
        name,
        first_item,
        amount
    );
    match db::add_budget_item(budget_id, name, &first_item, amount) {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error = %e, "Could not save new budget item");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
