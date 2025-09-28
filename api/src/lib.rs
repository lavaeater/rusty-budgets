//! This crate contains all shared fullstack server functions.

extern crate alloc;
extern crate core;

pub mod cqrs;
pub mod events;
pub mod import;
pub mod models;

use crate::models::*;
#[cfg(feature = "server")]
use dioxus::logger::tracing;
use dioxus::prelude::*;
use models::*;
use uuid::Uuid;

#[cfg(feature = "server")]
const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

#[cfg(feature = "server")]
pub mod db {
    use crate::cqrs::framework::Runtime;
    use crate::cqrs::runtime::{Db, JoyDbBudgetRuntime, UserBudgets};
    use crate::models::*;
    use crate::models::*;
    use crate::DEFAULT_USER_EMAIL;
    use chrono::NaiveDate;
    use dioxus::logger::tracing;
    use joydb::JoydbError;
    use once_cell::sync::Lazy;
    use uuid::Uuid;

    pub static CLIENT: Lazy<JoyDbBudgetRuntime> = Lazy::new(|| {
        tracing::info!("Init DB Client");
        let client = JoyDbBudgetRuntime::new("data.json");
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

    pub fn get_default_budget(user_id: &Uuid) -> anyhow::Result<Option<Budget>> {
        match with_client(None).get::<UserBudgets>(user_id) {
            Ok(b) => match b {
                None => {
                    tracing::info!("User has no budgets");
                    Ok(None)
                }
                Some(b) => match b.budgets.iter().find(|(_, default)| *default) {
                    Some((budget_id, _)) => match with_runtime(None).load(&budget_id) {
                        Ok(budget) => Ok(budget),
                        Err(_) => Err(anyhow::anyhow!("Could not load default budget")),
                    },
                    None => {
                        tracing::info!("User had budgets but none were default");
                        Ok(None)
                    }
                },
            },
            Err(_) => Err(anyhow::anyhow!("Could not get default budget")),
        }
    }

    pub fn add_budget_to_user(
        user_id: &Uuid,
        budget_id: &Uuid,
        default: bool,
    ) -> anyhow::Result<()> {
        match with_client(None).get::<UserBudgets>(&user_id) {
            Ok(list) => match list {
                None => {
                    match with_client(None).insert(&UserBudgets {
                        id: *user_id,
                        budgets: vec![(*budget_id, default)],
                    }) {
                        Ok(_) => Ok(()),
                        Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
                    }
                }
                Some(list) => {
                    if !list.budgets.contains(&(*budget_id, default)) {
                        let mut budgets = list.budgets.clone();
                        budgets.push((*budget_id, default));
                        let list = UserBudgets {
                            id: *user_id,
                            budgets,
                        };
                        match with_client(None).upsert(&list) {
                            Ok(_) => Ok(()),
                            Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
                        }
                    } else {
                        Ok(())
                    }
                }
            },
            Err(_) => Err(anyhow::anyhow!("Could not add budget to user")),
        }
    }

    pub fn create_budget(
        name: &str,
        user_id: &Uuid,
        default_budget: bool,
    ) -> anyhow::Result<Budget> {
        match with_runtime(None).create_budget(
            &name.to_string(),
            default_budget,
            Currency::SEK,
            *user_id,
        ) {
            Ok((budget, budget_id)) => {
                add_budget_to_user(user_id, &budget_id, default_budget)?;
                Ok(budget)
            }
            Err(_) => Err(anyhow::anyhow!("Could not create budget")),
        }
    }

    pub fn add_item(
        budget_id: &Uuid,
        user_id: &Uuid,
        name: &str,
        item_type: &BudgetingType,
        budgeted_amount: &Money,
    ) -> anyhow::Result<Budget> {
        with_runtime(None)
            .add_item(budget_id, name, item_type, budgeted_amount, user_id)
            .map(|(budget, _)| budget)
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
pub async fn create_budget(
    name: String,
    default_budget: Option<bool>,
) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::create_budget(&name, &user.id, default_budget.unwrap_or(true)) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn add_item(
    budget_id: Uuid,
    name: String,
    item_type: BudgetingType,
    budgeted_amount: Money,
) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::add_item(&budget_id, &user.id, &name, &item_type, &budgeted_amount) {
        Ok(b) => {
            // let items = b.budget_items.by_type(&item_type).expect("Could not get budgeting_type");
            Ok(b)
        }
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_user() -> Result<User, ServerFnError> {
    match db::get_default_user(None) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default User");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget(changed: bool) -> Result<Option<Budget>, ServerFnError> {
    tracing::info!("Changed: {}", changed);
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::get_default_budget(&user.id) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
