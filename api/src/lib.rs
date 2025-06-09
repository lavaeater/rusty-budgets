#![feature(string_remove_matches)]
extern crate core;

use crate::handlers::auth::setup_openid_client;
use crate::handlers::{auth, import, index, members, posts};
use entities::user;
use migration::{Migrator, MigratorTrait};
use poem::endpoint::StaticFilesEndpoint;
use poem::listener::TcpListener;
use poem::session::{CookieConfig, CookieSession};
use poem::{get, EndpointExt, Route, Server};
use sea_orm::prelude::Uuid;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, EntityTrait};
use serde::Deserialize;
use std::env;
use std::str::FromStr;
use tera::Tera;

mod handlers;

const DEFAULT_ITEMS_PER_PAGE: u64 = 100;

#[derive(Debug, Clone)]
struct AppState {
    templates: Tera,
    conn: DatabaseConnection,
}

#[derive(Deserialize, Default)]
struct PaginationParams {
    page: Option<u64>,
    items_per_page: Option<u64>,
}

#[tokio::main]
async fn start(root_path: Option<String>) -> std::io::Result<()> {
    let root_path = if let Some(root_path) = root_path {
        root_path
    } else {
        env::current_dir()?.to_str().unwrap().to_string()
    };
    tracing_subscriber::fmt::init();
    println!("Root path: {root_path}");

    // get env vars
    dotenvy::dotenv().ok();

    let db_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set in .env file");
    let host = env::var("HOST").expect("HOST is not set in .env file");
    let port = env::var("PORT").expect("PORT is not set in .env file");
    let server_url = format!("{host}:{port}");

    // create post table if not exists
    let conn = Database::connect(&db_url).await.unwrap();
    Migrator::up(&conn, None).await.unwrap();

    ensure_super_admin(&conn).await;
    let template_path = format!("{}/frontend/templates/**/*", &root_path);
    println!("{}", template_path);
    let templates = Tera::new(&template_path).unwrap();
    let google_client = setup_openid_client().await.unwrap();
    let state = AppState { templates, conn };

    println!("Starting server at {server_url}");
    let app = Route::new()
        .at("/", get(index::index))
        .nest("/posts", posts::post_routes())
        .nest("/members", members::member_routes())
        .nest("/auth", auth::routes())
        .nest("/import", import::import_routes())
        .nest(
            "/static",
            StaticFilesEndpoint::new(format!("{}/static", &root_path)),
        )
        .nest(
            "/dist",
            StaticFilesEndpoint::new(format!("{}/frontend/dist", &root_path)),
        )
        .with(CookieSession::new(CookieConfig::default())) //.secure(true)
        .data(state)
        .data(google_client);
    let server = Server::new(TcpListener::bind(format!("{host}:{port}")));
    server.run(app).await
}

async fn ensure_super_admin(database_connection: &DatabaseConnection) {
    let user_id = Uuid::from_str("920b2fc5-d127-4003-b3f9-43bb685558d4").unwrap();
    if let Ok(Some(_user)) = user::Entity::find_by_id(user_id.clone())
        .one(database_connection)
        .await
    {
        return;
    }

    let _u = user::ActiveModel {
        id: Set(user_id),
        email: Set("tommie.nygren@gmail.com".to_string()),
        name: Set("Tommie Nygren".to_string()),
        role: Set("super_admin".to_string()),
    }
    .insert(database_connection)
    .await;
}

pub fn main(root_path: Option<String>) {
    let result = start(root_path);

    if let Some(err) = result.err() {
        println!("Error: {err}");
    }
}
