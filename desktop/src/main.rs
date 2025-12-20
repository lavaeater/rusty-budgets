#![allow(unused_imports)]
use dioxus::logger::tracing::Level;
use dioxus::prelude::*;
use dioxus::fullstack;
mod views;
use views::*;
const MAIN_CSS: Asset = asset!("/assets/main.css");

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(DesktopNavbar)]
    #[route("/")]
    Home {},    
    #[route("/:..segments")]
    PageNotFound { segments: Vec<String> },
}

fn main() {
    dioxus::logger::init(Level::INFO).expect("failed to init logger");
    // #[cfg(not(feature = "server"))]
    // fullstack::set_server_url("http://localhost");    
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
        // Navbar {
        //     Link { to: Route::Home {}, "Översikt" }
        //     Link { to: Route::Blog { id: 1 }, "Blog" }
        // }

        Outlet::<Route> {}
    }
}

#[component]
fn PageNotFound(segments: Vec<String>) -> Element {
    rsx! {
        h1 { "404" }
    }
}
