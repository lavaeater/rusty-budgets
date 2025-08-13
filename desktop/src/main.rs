use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use uuid::Uuid;
use ui::Navbar;
use views::{Blog, Home, NewBudgetItem, PageNotFound, Budget};
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(DesktopNavbar)]
        #[route("/")]
        Home {},
        #[route("/blog/:id")]
        Blog { id: i32 },
    #[end_layout]
    #[nest("/budget")]
        #[route("/:id")]
        Budget {
            id: Uuid
        },
        #[nest("/:budget_id")]
            #[route("/new_budget_item/:item_type")]
            NewBudgetItem {
                // You must include parent dynamic segments in child variants
                budget_id: Uuid,
                item_type: String
            },
    // End nests manually with #[end_nest]
        #[end_nest]
    #[end_nest]
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    
    #[cfg(feature = "server")]
    let _ = api::db::CLIENT;
    
    launch(App);
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
