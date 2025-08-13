use dioxus::prelude::*;
use uuid::Uuid;
use ui::BudgetOverview;

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
        BudgetOverview { id: id }
    }
}

const _BUDGET_CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn NewBudgetItem(budget_id: Uuid, item_type: String) -> Element {
    rsx! {
        ui::NewBudgetItem { budget_id: budget_id, item_type: item_type }
    }
}