//! This crate contains all shared fullstack server functions.
mod migrations;
mod models;

use crate::models::user::User;
use dioxus::logger::tracing;
use dioxus::prelude::*;

const DEFAULT_USER_EMAIL: &str = "tommie.nygren@gmail.com";

#[cfg(feature = "server")]
pub mod db {
    use crate::models::user::User;
    use crate::{DEFAULT_USER_EMAIL, migrations};
    use dioxus::logger::tracing;
    use dioxus::prelude::ServerFnError;
    use once_cell::sync::Lazy;
    use sqlx::types::chrono::NaiveDate;
    use sqlx::types::uuid;
    use welds::connections::any::AnyClient;
    use welds::state::DbState;
    use welds::{WeldsError, errors};
    use crate::models::budget::Budget;

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

            if user_exists(DEFAULT_USER_EMAIL, Some(&client)).await {
                tracing::info!("Default user already exists");
            }

            if let Ok(res) = User::all()
                .where_col(|u| u.email.equal(DEFAULT_USER_EMAIL))
                .run(&client)
                .await
            {
                if res.is_empty() {
                    let mut user = DbState::new_uncreated(User {
                        id: uuid::Uuid::new_v4(),
                        first_name: "Tommie".to_string(),
                        last_name: "Nygren".to_string(),
                        phone: Some("+46|0704382781".to_string()),
                        email: DEFAULT_USER_EMAIL.to_string(),
                        user_name: "tommie".to_string(),
                        birthday: Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    });
                    user.save(&client).await.unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Could not create default user");
                    });
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

    pub async fn list_users(client: Option<&AnyClient>) -> errors::Result<Vec<DbState<User>>> {
        User::all().run(client_from_option(client)).await
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

    pub async fn get_default_user(client: Option<&AnyClient>) -> errors::Result<DbState<User>> {
        User::all()
            .where_col(|u| u.email.equal(DEFAULT_USER_EMAIL))
            .fetch_one(client_from_option(client))
            .await
    }
    
    pub async fn get_default_budget_for_user(user_id: uuid::Uuid, client: Option<&AnyClient>) -> errors::Result<DbState<Budget>> {
        match Budget::all()
            .where_col(|b| b.user_id.equal(user_id))
            .where_col(|b| b.default.equal(true))
            .fetch_one(client_from_option(client))
            .await {
            Ok(budget) => { Ok(budget) },
            Err(_) => {}
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
    ) -> errors::Result<DbState<User>> {
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
            Ok(_) => Ok(user),
            Err(e) => {
                tracing::error!(error = %e, "Could not create user");
                Err(WeldsError::Other(e.into()))
            }
        }
    }
}

/// Echo the user input on the server.
#[server(Echo)]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    match db::list_users(None).await {
        Ok(users) => Ok(users.into_iter().map(|u| u.into_inner()).collect()),
        Err(e) => {
            tracing::error!(error = %e, "Could not list users");
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}
