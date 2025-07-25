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
pub fn BudgetOverview(id: Uuid) -> Element {
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
}