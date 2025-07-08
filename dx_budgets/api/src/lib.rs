//! This crate contains all shared fullstack server functions.
mod migrations;
mod models;

use dioxus::prelude::*;
use crate::models::user::User;

#[cfg(feature = "server")]
pub mod db {
    use crate::migrations;
    use crate::models::user::User;
    use once_cell::sync::Lazy;
    use sqlx::types::chrono::NaiveDate;
    use sqlx::types::uuid;
    use welds::connections::any::AnyClient;
    use welds::state::DbState;

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
                        user_name: "tommie".to_string(),
                        birthday: Some(
                            NaiveDate::parse_from_str("1973-05-12", "%Y-%m-%d").unwrap_or_default(),
                        ),
                    });
                    user.save(&client).await.expect("Could not save user");
                }
            }
            client
        })
    });
    
    pub async fn list_users() -> Option::<Vec<User>> {
        match User::all().run(CLIENT.as_ref()).await {
            Ok(users) => Some(users.into_iter().map(|u| u.into_inner()).collect()),
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }
}

/// Echo the user input on the server.
#[server(Echo)]
pub async fn list_users() -> Result<Vec<User>, ServerFnError> {
    match db::list_users().await {
        None => {Ok(Vec::new())}
        Some(users) => {Ok(users)}
    }
}
