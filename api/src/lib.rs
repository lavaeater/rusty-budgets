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
        let budget = Budget::new("Test Budget".to_string(),  user_id, true);
        
        
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
