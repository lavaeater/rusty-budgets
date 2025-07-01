//! This crate contains all shared fullstack server functions.

mod migrations;
mod models;

use dioxus::prelude::*;
use welds::connections::any::AnyClient;
use welds::prelude::*;

use tokio::task_local;

task_local! {
    static DB_CLIENT: AnyClient;
}

/// Echo the user input on the server.
#[server(Echo)]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    let connection_string = "sqlite::./database.sqlite";
    let client = welds::connections::connect(connection_string).await?;
    Ok(input)
}

#[derive(Clone, Copy, Debug)]
pub struct DatabasePool;

#[server]
pub async fn my_wacky_server_fn(input: Vec<String>) -> Result<String, ServerFnError> {
    let FromContext(pool): FromContext<DatabasePool> = extract().await?;
    Ok(format!("The server read {:?} from the shared context", pool))
}


// The database is only available to server code
    async fn db() -> Result<AnyClient, ServerFnError> {
        
        let connection_string = "sqlite::./database.sqlite";
        let client =welds::connections::connect(connection_string).await?;
        Ok(client)
        // Return the connection 
    }