use dioxus::logger::tracing;
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use ui::Navbar;
use views::{Blog, Home};
mod views;
// use dioxus_provider::global::init_global_providers;

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
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    // init_global_providers();

    #[cfg(feature = "server")]
    let _ = api::db::CLIENT.as_ref();
    
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
