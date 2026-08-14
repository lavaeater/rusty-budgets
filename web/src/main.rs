#[cfg(not(feature = "server"))]
use dioxus::fullstack;
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use views::Budget;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(WebNavbar)]
    #[redirect("/", || {
        let now = ui::budget::BudgetLocation::current();
        Route::Budget { period: now.period_slug(), tab: now.tab_slug().to_string() }
    })]
    #[route("/budget/:period/:tab")]
    Budget { period: String, tab: String },
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");

    #[cfg(not(feature = "server"))]
    fullstack::set_server_url("http://127.0.0.1");

    launch(App);
}

#[component]
fn App() -> Element {
    // Build cool things ✌️

    rsx! {
        // Global app resources
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// A web-specific Router around the shared `Navbar` component
/// which allows us to use the web-specific `Route` enum.
#[component]
fn WebNavbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
