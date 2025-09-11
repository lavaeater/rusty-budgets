//! This crate contains all shared fullstack server functions.
pub mod models;
mod cqrs;
mod runtime;

#[cfg(feature = "server")]
use dioxus::logger::tracing;
use crate::models::*;
use dioxus::prelude::*;
use crate::cqrs::budget::Budget;

#[cfg(feature = "server")]
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

#[cfg(feature = "server")]
pub mod db {
    use crate::models::*;
    use crate::{DEFAULT_USER_EMAIL};
    use crate::runtime::{Db, JoyDbBudgetRuntime};
    use chrono::NaiveDate;
    use dioxus::logger::tracing;
    use uuid::Uuid;
    use once_cell::sync::Lazy;
    use crate::cqrs::budget::Budget;

    pub static CLIENT: Lazy<JoyDbBudgetRuntime> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = JoyDbBudgetRuntime::new();
        // Run migrations
        tracing::info!("Insert Default Data");
        match get_default_user(Some(&client.db)) {
            Ok(user) => {
                tracing::info!("Default user exists");
            }
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                panic!("Could not get default user");
            }
        }
        client
    });

    fn with_client(client: Option<&Db>) -> &Db {
        if let Some(c) = client {
            c
        } else {
            &CLIENT.db
        }
    }

    fn with_runtime(client: Option<&JoyDbBudgetRuntime>) -> &JoyDbBudgetRuntime {
        if let Some(c) = client {
            c
        } else {
            &CLIENT
        }
    }
    
    pub fn user_exists(email: &str, client: Option<&Db>) -> anyhow::Result<bool> {
        match with_client(client).get_all_by(|u: &User| u.email == email) {
            Ok(users) => Ok(!users.is_empty()),
            Err(e) => {
                tracing::error!(error = %e, "Could not get default user");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub fn get_default_user(client: Option<&Db>) -> anyhow::Result<User> {
        match with_client(client).get_all_by(|u: &User| u.email == DEFAULT_USER_EMAIL) {
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

    pub fn create_budget(
        name: &str,
        user_id: Uuid,
        default_budget: bool,
        runtime: Option<&JoyDbBudgetRuntime>,
    ) -> anyhow::Result<Budget> {
        let budget_id = Uuid::new_v4();
        with_runtime(runtime).cmd(budget_id, |budget| budget.create_budget(name.to_string(), user_id, default_budget))
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
        match with_client(client).insert(&user) {
            Ok(_) => Ok(user),
            Err(e) => {
                tracing::error!(error = %e, "Could not create user");
                Err(anyhow::Error::from(e))
            }
        }
    }
}

#[server]
pub async fn create_budget(name: String, default_budget: Option<bool>) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::create_budget(&name, user.id, default_budget.unwrap_or(true), None) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
