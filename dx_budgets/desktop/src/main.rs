use dioxus::prelude::*;

use ui::Navbar;
use views::{Blog, Home};
mod views;
use api::DatabasePool;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(DesktopNavbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

#[tokio::main]
async fn main() {
    // Initialize database pool
    let connection_string = "sqlite::./database.sqlite".to_string();
    let db_pool = DatabasePool::new(connection_string);
    
    // Run migrations on startup
    if let Err(e) = db_pool.initialize_with_migrations().await {
        eprintln!("Failed to initialize database: {}", e);
        std::process::exit(1);
    }
    
    println!("Database initialized successfully with migrations");
    
    LaunchBuilder::new()
        .with_context(server_only! {
            db_pool
        })
        .launch(App);
}

#[component]
fn App() -> Element {
    // Build cool things 

    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// A desktop-specific Router around the shared `Navbar` component
/// which allows us to use the desktop-specific `Route` enum.
#[component]
fn DesktopNavbar() -> Element {
    rsx! {
        Navbar {
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Blog { id: 1 },
                "Blog"
            }
        }

        Outlet::<Route> {}
    }
}
