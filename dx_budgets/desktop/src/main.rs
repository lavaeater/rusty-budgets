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

fn main() {
    // Initialize database pool
    let connection_string = "sqlite://./database.sqlite".to_string();
    let db_pool = DatabasePool::new(connection_string);
    
    // Initialize database with migrations synchronously
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client = rt.block_on(async {
        db_pool.initialize_with_migrations().await.unwrap();
        db_pool.get_connection().await.unwrap()
    });
    
    println!("Database initialized successfully with migrations");
    
    LaunchBuilder::new()
        .with_context(server_only! {
            client
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
