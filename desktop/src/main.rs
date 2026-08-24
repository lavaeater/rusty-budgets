#[cfg(not(feature = "server"))]
use dioxus::fullstack;
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
#[cfg(not(feature = "server"))]
use std::env;
use views::Budget;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(DesktopNavbar)]
    #[redirect("/", || {
        let now = ui::budget::BudgetLocation::current();
        Route::Budget { period: now.period_slug(), tab: now.tab_slug().to_string() }
    })]
    #[route("/budget/:period/:tab")]
    Budget { period: String, tab: String },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    #[cfg(not(feature = "server"))]
    {
        let server_url = env::var("SERVER_URL").unwrap_or("http://localhost".to_string());
        let port = env::var("PORT").unwrap_or("8080".to_string());
        fullstack::set_server_url(Box::leak(format!("{server_url}:{port}").into_boxed_str()));
    }

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
        Outlet::<Route> {}
    }
}
