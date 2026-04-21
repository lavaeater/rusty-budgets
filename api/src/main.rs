use std::env;
use welds::prelude::*;

mod errors;
mod migrations;
mod models;

#[tokio::main]
async fn main() -> welds::errors::Result<()> {
    // Read .env file and setup logging
    if let Err(err) = dotenvy::dotenv() {
        match err {
            dotenvy::Error::Io(_) => {}
            _ => eprintln!("DOTENV: {:?}", err),
        }
    }
    pretty_env_logger::init();

    // Connect to the database and run the migrations
    let connection_string = env::var("DATABASE_URL").unwrap(); // default value in .env file
    let client = welds::connections::connect(&connection_string)
        .await
        .expect("Unable to connect to Database");
    migrations::up(&client).await.unwrap();

    Ok(())
}
