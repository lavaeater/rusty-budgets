//! This crate contains all shared fullstack server functions.
mod migrations;
pub mod models;

use crate::models::budget::Budget;
use crate::models::user::User;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use serde::Serialize;
use uuid::Uuid;

const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

#[cfg(feature = "server")]
pub mod db {
    use crate::models::budget::Budget;
    use crate::models::user::User;
    use crate::{DEFAULT_USER_EMAIL, migrations};
    use dioxus::hooks::use_signal;
    use dioxus::logger::tracing;
    use dioxus::prelude::{ServerFnError, Signal, UnsyncStorage};
    use once_cell::sync::Lazy;
    use sqlx::types::chrono::NaiveDate;
    use sqlx::types::uuid;
    use std::future::Future;
    use uuid::Uuid;
    use welds::connections::any::AnyClient;
    use welds::state::DbState;
    use welds::{WeldsError, errors};

    pub static CLIENT: Lazy<AnyClient> = Lazy::new(|| {
        tracing::info!("Init DB Client");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Create DB Client");
            let client = welds::connections::connect("sqlite://./database.sqlite?mode=rwc")
                .await
                .expect("Could not create Client");
            // Run migrations
            tracing::info!("Run migrations");
            migrations::up(&client)
                .await
                .expect("Could not run migrations");

            match get_default_user(Some(&client)).await {
                Ok(user) => {
                    tracing::info!("Default user exists");
                    match get_default_budget_for_user(user.id, Some(&client)).await {
                        Ok(_) => {
                            tracing::info!("Default budget exists");
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
        })
    });

    fn client_from_option(client: Option<&AnyClient>) -> &AnyClient {
        if let Some(c) = client {
            c
        } else {
            CLIENT.as_ref()
        }
    }

    pub async fn list_users(client: Option<&AnyClient>) -> anyhow::Result<Vec<User>> {
        match User::all().run(client_from_option(client)).await {
            Ok(users) => Ok(users.into_iter().map(|u| u.into_inner()).collect()),
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }

    pub async fn user_exists(email: &str, client: Option<&AnyClient>) -> bool {
        tracing::info!("user_exists");
        if let Ok(res) = User::all()
            .where_col(|u| u.email.equal(email))
            .run(client_from_option(client))
            .await
        {
            tracing::info!("user_exists: {}", !res.is_empty());
            !res.is_empty()
        } else {
            tracing::info!("user_exists: false, an error occurred");
            false
        }
    }

    pub async fn get_default_user(client: Option<&AnyClient>) -> anyhow::Result<User> {
        match User::all()
            .where_col(|u| u.email.equal(DEFAULT_USER_EMAIL))
            .fetch_one(client_from_option(client))
            .await
        {
            Ok(u) => Ok(u.into_inner()),
            Err(e) => match e {
                WeldsError::RowNowFound => {
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
                    .await
                }
                _ => {
                    tracing::error!(error = %e, "Could not get default user");
                    Err(anyhow::Error::from(e))
                }
            },
        }
    }

    /***
    I am totally done with this: we need to load the budget and modify
    it OR return tracked entities to the ui, which might be cool as well.

    We'll figure it out, bro
     */

    pub async fn save_budget(budget: Budget) -> anyhow::Result<()> {
        let mut budget_to_save = DbState::db_loaded(Budget::default());
        budget_to_save.replace_inner(budget);
        match budget_to_save.save(client_from_option(None)).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!(error = %e, "Could not save budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub async fn get_default_budget_for_user(
        user_id: uuid::Uuid,
        client: Option<&AnyClient>,
    ) -> anyhow::Result<Budget> {
        match Budget::all()
            .where_col(|b| b.user_id.equal(user_id))
            .where_col(|b| b.default_budget.equal(true))
            .fetch_one(client_from_option(client))
            .await
        {
            Ok(b) => Ok(b.into_inner()),
            Err(e) => match e {
                WeldsError::RowNowFound => {
                    tracing::info!("No default budget exists, time to create one");
                    create_budget("Default", user_id, true, client).await
                }
                _ => {
                    tracing::error!(error = %e, "Could not get default budget for user");
                    Err(anyhow::Error::from(e))
                }
            },
        }
    }

    pub async fn create_budget(
        name: &str,
        user_id: uuid::Uuid,
        default_budget: bool,
        client: Option<&AnyClient>,
    ) -> anyhow::Result<Budget> {
        let mut budget = DbState::new_uncreated(Budget {
            id: Uuid::default(),
            name: name.to_string(),
            user_id,
            default_budget,
            created_at: Default::default(),
            updated_at: Default::default(),
        });
        match budget.save(client_from_option(client)).await {
            Ok(_) => Ok(budget.into_inner()),
            Err(e) => {
                tracing::error!(error = %e, "Could not create budget");
                Err(anyhow::Error::from(e))
            }
        }
    }

    pub async fn create_user(
        user_name: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        phone: Option<String>,
        birthday: Option<NaiveDate>,
        client: Option<&AnyClient>,
    ) -> anyhow::Result<User> {
        let mut user = DbState::new_uncreated(User {
            id: uuid::Uuid::new_v4(),
            first_name: first_name.to_string(),
            last_name: last_name.to_string(),
            phone,
            email: email.to_string(),
            user_name: user_name.to_string(),
            birthday,
        });
        match user.save(client_from_option(client)).await {
            Ok(_) => Ok(user.into_inner()),
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
    match db::list_users(None).await {
        Ok(users) => Ok(users),
        Err(e) => {
            tracing::error!(error = %e, "Could not list users");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn get_default_budget() -> Result<Budget, ServerFnError> {
    match db::get_default_budget_for_user(db::get_default_user(None).await.unwrap().id, None).await
    {
        Ok(budget) => Ok(budget),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[server]
pub async fn save_budget(budget: Budget) -> Result<(), ServerFnError> {
    match db::save_budget(budget).await {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::error!(error = %e, "Could not get default budget");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
