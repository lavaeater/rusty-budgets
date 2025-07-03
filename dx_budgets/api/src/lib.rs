//! This crate contains all shared fullstack server functions.

mod migrations;
mod models;

use crate::models::user::User;
use dioxus::prelude::*;
use welds::connections::any::AnyClient;

pub mod db {
    use crate::migrations;
    use crate::models::user::User;
    use dioxus::prelude::ServerFnError;
    use once_cell::sync::Lazy;
    use sqlx::types::chrono::NaiveDate;
    use sqlx::types::uuid;
    use welds::connections::any::AnyClient;
    use welds::migrations::types::Type::Uuid;
    use welds::state::DbState;
    
    pub static c: AnyClient = {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = welds::connections::connect("sqlite://./database.sqlite")
                .await
                .expect("Could not create Client");
            // Run migrations
            migrations::up(&client)
                .await
                .expect("Could not run migrations");

            if let Ok(res) = User::all()
                .where_col(|u| u.email.equal("tommie.nygren@gmail.com"))
                .run(&client)
                .await
            {
                if res.is_empty() {
                    let mut user = DbState::new_uncreated(User {
                        id: uuid::Uuid::new_v4(),
                        first_name: "Tommie".to_string(),
                        last_name: "Nygren".to_string(),
                        phone: Some("+46|0704382781".to_string()),
                        email: "tommie.nygren@gmail.com".to_string(),
                        username: "tommie".to_string(),
                        birthday: Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    });
                    user.save(&client).await;
                }
            }
            client
        })
    };

    pub static CLIENT: Lazy<AnyClient> = Lazy::new(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let client = welds::connections::connect("sqlite://./database.sqlite")
                .await
                .expect("Could not create Client");
            // Run migrations
            migrations::up(&client)
                .await
                .expect("Could not run migrations");

            if let Ok(res) = User::all()
                .where_col(|u| u.email.equal("tommie.nygren@gmail.com"))
                .run(&client)
                .await
            {
                if res.is_empty() {
                    let mut user = DbState::new_uncreated(User {
                        id: uuid::Uuid::new_v4(),
                        first_name: "Tommie".to_string(),
                        last_name: "Nygren".to_string(),
                        phone: Some("+46|0704382781".to_string()),
                        email: "tommie.nygren@gmail.com".to_string(),
                        username: "tommie".to_string(),
                        birthday: Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    });
                    user.save(&client).await;
                }
            }
            client
        })
    });

    pub async fn init_client() -> AnyClient {
        welds::connections::connect("sqlite://./database.sqlite")
            .await
            .expect("Could not create Client")
    }
}

/// Echo the user input on the server.
#[server(Echo)]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    let client = db::CLIENT.as_ref();
    let users = User::all().run(client).await?;
    if users.len() > 0 {
        return Ok(format!(
            "The server read {:?} from the shared context with database pool",
            input
        ));
    } else {
        return Ok("Gronk".to_string());
    }
    Ok(input)
}

#[derive(Clone, Debug)]
pub struct DatabasePool {
    connection_string: String,
}

impl DatabasePool {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }

    pub async fn get_connection(&self) -> Result<AnyClient, ServerFnError> {
        let client = welds::connections::connect(&self.connection_string).await?;
        Ok(client)
    }

    pub async fn initialize_with_migrations(&self) -> Result<(), ServerFnError> {
        let client = self.get_connection().await?;

        // Run migrations
        migrations::up(&client)
            .await
            .map_err(|e| ServerFnError::new(format!("Migration failed: {}", e)))?;

        Ok(())
    }
}

#[server]
pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<String, ServerFnError> {
    let FromContext(client): FromContext<AnyClient> = extract().await?;
    let users = User::all().run(&client).await?;
    Ok(format!(
        "The server read {:?} from the shared context with database pool",
        input
    ))
}

#[server]
pub async fn get_database_info() -> Result<String, ServerFnError> {
    let FromContext(pool): FromContext<DatabasePool> = extract().await?;
    let connection = pool.get_connection().await?;

    // Example: You can now use the connection for database operations
    // For now, just return connection info
    Ok(format!(
        "Database connection established successfully to: {}",
        pool.connection_string
    ))
}
