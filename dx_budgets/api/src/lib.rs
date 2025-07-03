//! This crate contains all shared fullstack server functions.

mod migrations;
mod models;

use dioxus::prelude::*;
use welds::connections::any::AnyClient;
use crate::models::user::User;

/// Echo the user input on the server.
#[server(Echo)]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    #[cfg(feature = "server")]
    {
        let FromContext(client): FromContext<AnyClient> = extract().await?;
        let users = User::all().run(&client).await?;
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
        migrations::up(&client).await.map_err(|e| {
            ServerFnError::new(format!("Migration failed: {}", e))
        })?;
        
        Ok(())
    }
}

#[server]
pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<String, ServerFnError> {
    let FromContext(client): FromContext<AnyClient> = extract().await?;
    let users = User::all().run(&client).await?;
    Ok(format!("The server read {:?} from the shared context with database pool", input))
}

#[server]
pub async fn get_database_info() -> Result<String, ServerFnError> {
    let FromContext(pool): FromContext<DatabasePool> = extract().await?;
    let connection = pool.get_connection().await?;
    
    // Example: You can now use the connection for database operations
    // For now, just return connection info
    Ok(format!("Database connection established successfully to: {}", pool.connection_string))
}