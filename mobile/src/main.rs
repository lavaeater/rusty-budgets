use dioxus::logger::tracing::Level;
use dioxus::prelude::*;

use views::Home;

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MobileNavbar)]
    #[route("/")]
    Home {},
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
    // Build cool things ✌️

    rsx! {
        // Global app resources
        document::Link { rel: "stylesheet", href: MAIN_CSS }

        Router::<Route> {}
    }
}

/// A mobile-specific Router around the shared `Navbar` component
/// which allows us to use the mobile-specific `Route` enum.
#[component]
fn MobileNavbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
