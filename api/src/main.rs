use std::env;
use api::api_error::RustyError;
use api::cqrs::runtime::{migrate_to_postgres};

#[tokio::main]
async fn main() -> Result<(), RustyError> {
    let migrate = true;
    if let Err(err) = dotenvy::dotenv() {
        match err {
            dotenvy::Error::Io(_) => {}
            _ => eprintln!("DOTENV: {:?}", err),
        }
    }
    pretty_env_logger::init();

    let connection_string = env::var("DATABASE_URL").unwrap();
    let client = welds::connections::connect(&connection_string)
        .await
        .expect("Unable to connect to Database");
    api::migrations::up(&client).await?;
    if migrate {
        migrate_to_postgres().await?;        
    }

    Ok(())
}
