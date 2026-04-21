use std::env;

#[tokio::main]
async fn main() -> welds::errors::Result<()> {
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
    api::migrations::up(&client).await.unwrap();

    Ok(())
}
