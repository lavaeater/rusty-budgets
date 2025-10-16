//! This crate contains all shared fullstack server functions.
#![allow(unused_imports)]
#![allow(dead_code)]
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
    use crate::cqrs::framework::{CommandError, Runtime};
    use crate::cqrs::runtime::{Db, JoyDbBudgetRuntime, UserBudgets};
    use crate::events::TransactionConnected;
    use crate::models::*;
    use crate::models::*;
    use crate::DEFAULT_USER_EMAIL;
    use anyhow::Error;
    use chrono::NaiveDate;
    use dioxus::logger::tracing;
    use joydb::JoydbError;
    use once_cell::sync::Lazy;
    use uuid::Uuid;
    use crate::import::import_from_skandia_excel;

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

    pub fn get_budget(budget_id: &Uuid) -> anyhow::Result<Budget> {
        match with_runtime(None).load(&budget_id) {
            Ok(budget) => match budget {
                None => Err(anyhow::anyhow!("Could not load budget")),
                Some(budget) => Ok(budget),
            },
            Err(_) => Err(anyhow::anyhow!("Could not load budget")),
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

    pub fn import_transactions(
        budget_id: &Uuid,
        user_id: &Uuid,
        file_name: &str,
    ) -> anyhow::Result<Budget> {
        let runtime = with_runtime(None);
        let _ = import_from_skandia_excel(file_name, user_id, budget_id, runtime)?;
        get_budget(budget_id)
    }

    pub fn add_item(
        budget_id: &Uuid,
        user_id: &Uuid,
        name: &str,
        item_type: &BudgetingType,
        budgeted_amount: &Money,
    ) -> anyhow::Result<(Budget, Uuid)> {
        with_runtime(None).add_item(budget_id, name, item_type, budgeted_amount, user_id)
    }

    pub fn modify_item(
        budget_id: &Uuid,
        item_id: &Uuid,
        user_id: &Uuid,
        name: Option<String>,
        item_type: Option<BudgetingType>,
        budgeted_amount: Option<Money>,
    ) -> anyhow::Result<Budget> {
        with_runtime(None).modify_item(
            budget_id,
            item_id,
            name,
            item_type,
            budgeted_amount,
            None,
            None,
            user_id,
        )
        .map(|(b, _)| b)
    }

    pub fn connect_transaction(
        budget_id: &Uuid,
        user_id: &Uuid,
        tx_id: &Uuid,
        item_id: &Uuid,
    ) -> anyhow::Result<Budget> {
        match with_runtime(None).connect_transaction(budget_id, tx_id, item_id, user_id) {
            Ok((budget, _)) => {
                let budget = create_rule(&budget, user_id, tx_id, item_id).unwrap_or(budget);
                let matches = budget.evaluate_rules();
                let mut return_budget = budget.clone();
                for (tx_id, item_id) in matches {
                    match with_runtime(None)
                        .connect_transaction(budget_id, &tx_id, &item_id, user_id)
                    {
                        Ok((b, _)) => {
                            return_budget = b;
                            tracing::info!("connceted some tranny");
                        }
                        Err(e) => {
                            tracing::error!("failed to connect the tranny {}", e);
                        }
                    }
                }
                Ok(return_budget)
            }
            Err(err) => Err(err),
        }
    }

    pub fn ignore_transaction(
        budget_id: &Uuid,
        user_id: &Uuid,
        tx_id: &Uuid,
    ) -> anyhow::Result<Budget> {
        with_runtime(None)
            .ignore_transaction(budget_id, tx_id, user_id)
            .map(|(b, _)| b)
    }

    pub fn create_rule(
        budget: &Budget,
        user_id: &Uuid,
        tx_id: &Uuid,
        item_id: &Uuid,
    ) -> anyhow::Result<Budget> {
        let transaction = budget.get_transaction(tx_id).unwrap();
        let item = budget.get_item(item_id).unwrap();
        let transaction_key = MatchRule::create_transaction_key(transaction);
        let item_name = item.name.clone();
        let always_apply = true;

        with_runtime(None)
            .add_rule(
                &budget.id,
                transaction_key,
                item_name,
                always_apply,
                user_id,
            )
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
) -> Result<(Budget, Uuid), ServerFnError> {
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
pub async fn modify_item(
    budget_id: Uuid,
    item_id: Uuid,
    name: Option<String>,
    item_type: Option<BudgetingType>,
    budgeted_amount: Option<Money>,
) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::modify_item(&budget_id, &item_id, &user.id, name, item_type, budgeted_amount) {
        Ok(b) => {
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
pub async fn get_default_budget() -> Result<Option<Budget>, ServerFnError> {
    get_budget(None)
}

#[server]
pub async fn get_budget(budget_id: Option<Uuid>) -> Result<Option<Budget>, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    if let Some(budget_id) = budget_id {
        match db::get_budget(&budget_id) {
            Ok(b) => Ok(b),
            Err(e) => {
                tracing::error!(error = %e, "Could not get budget");
                Err(ServerFnError::ServerError(e.to_string()))
            }
        }
    } else {
        match db::get_default_budget(&user.id) {
            Ok(b) => Ok(b),
            Err(e) => {
                tracing::error!(error = %e, "Could not get default budget");
                Err(ServerFnError::ServerError(e.to_string()))
            }
        }
    }
}

#[server]
pub async fn import_transactions(
    budget_id: Uuid,
    file_name: String,
) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::import_transactions(&budget_id, &user.id, &file_name) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn connect_transaction(
    budget_id: Uuid,
    tx_id: Uuid,
    item_id: Uuid,
) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::connect_transaction(&budget_id, &user.id, &tx_id, &item_id) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not connect transaction to item.");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn ignore_transaction(budget_id: Uuid, tx_id: Uuid) -> Result<Budget, ServerFnError> {
    let user = db::get_default_user(None).expect("Could not get default user");
    match db::ignore_transaction(&budget_id, &user.id, &tx_id) {
        Ok(b) => Ok(b),
        Err(e) => {
            tracing::error!(error = %e, "Could not ignore transaction to item.");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
