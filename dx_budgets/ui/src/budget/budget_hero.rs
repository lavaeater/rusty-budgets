use dioxus::prelude::*;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");


#[component]
pub fn BudgetHero() -> Element {
    let budgets = use_server_future(|| api::get_default_budget())?().unwrap().unwrap();
    rsx! {
        document::Link { rel: "stylesheet", href: BUDGET_CSS }
        div {
            id: "budget_hero",
            h4 { "{budgets.name}" }
        }
    }
}

