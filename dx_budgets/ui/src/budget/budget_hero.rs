use dioxus::prelude::*;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");


#[component]
pub fn BudgetHero() -> Element {
    let mut edit_budget_name = use_signal(|| false);
    let budget = use_server_future(|| api::get_default_budget())?().unwrap()?;
    rsx! {
        document::Link { rel: "stylesheet", href: BUDGET_CSS }
        div {
            id: "budget_hero"
            if edit_budget_name {
                input {
                    type: "text",
                    value: "{budget.name}",
                }
            } else {
                h4 {
                    "{budget.name}",
                    onclick: move |_| {
                        edit_budget_name.set(true);
                    }
                }
            }
        }
    }
}

