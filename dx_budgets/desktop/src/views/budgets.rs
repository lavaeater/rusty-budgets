use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;
use crate::Route;

#[component]
pub fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre { color: "red", "log:\nattemped to navigate to: {route:?}" }
    }
}

#[component]
pub fn Budget(id: Uuid) -> Element {
    rsx! {
        h1 { "Budget" }
    }
}

const BUDGET_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn NewBudgetItem(budget_id: Uuid) -> Element {
    rsx! {
        ui::NewBudgetItem { budget_id: budget_id }
    }
    // tracing::info!("budget_id: {budget_id}");
    // rsx! {
    //     document::Link { rel: "stylesheet", href: BUDGET_CSS}
    // 
    //     div {
    //         id: "blog",
    // 
    //         // Content
    //         h1 { "This is blog!" }
    //         p { "In blog, we show how the Dioxus router works and how URL parameters can be passed as props to our route components." }
    // 
    //         span { " <---> " }
    //     }
    // }
}