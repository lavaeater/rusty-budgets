#[cfg(not(feature = "server"))]
use dioxus::fullstack;
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use views::Budget;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MobileNavbar)]
    #[redirect("/", || {
        let now = ui::budget::BudgetLocation::current();
        Route::Budget { period: now.period_slug(), tab: now.tab_slug().to_string() }
    })]
    #[route("/budget/:period/:tab")]
    Budget { period: String, tab: String },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

/// Import/export payloads (zip files with attached documents) can comfortably exceed axum's
/// default 2MB request body limit, so we raise it for the whole app.
#[cfg(feature = "server")]
const MAX_BODY_SIZE: usize = 100 * 1024 * 1024;

fn main() {
    // Route Rust logging to logcat so `adb logcat -s rusty_budgets` shows our
    // diagnostics. Dioxus does not install an Android logger itself, and the
    // native `println!` used elsewhere never reaches logcat — without this the
    // mobile build is effectively un-debuggable.
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("rusty_budgets"),
    );

    dioxus::logger::init(Level::INFO).expect("failed to init logger");

    // The Android WebView's "localhost" is the *device's* loopback, not the dev
    // machine's — it only resolves to the fullstack server via `adb reverse
    // tcp:8080 tcp:8080`, which forwards the device's localhost:8080 back to
    // the host. See mobile/README.md for the one-time setup.
    #[cfg(not(feature = "server"))]
    fullstack::set_server_url("http://localhost:8080");

    #[cfg(feature = "server")]
    dioxus::server::serve(|| async move {
        Ok(dioxus::server::router(App)
            .layer(dioxus::server::axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE)))
    });

    #[cfg(not(feature = "server"))]
    launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Global app resources
        document::Meta { name: "viewport", content: "width=device-width, initial-scale=1, viewport-fit=cover" }
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
