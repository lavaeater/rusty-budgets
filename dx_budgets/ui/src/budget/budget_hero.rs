use dioxus::prelude::*;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");


#[component]
pub fn BudgetHero() -> Element {
    let mut edit_budget_name = use_signal(|| false);
    let mut budget = use_server_future(|| api::get_default_budget())?().unwrap().unwrap();
    let mut b_name = use_signal(|| budget.name.clone());
    rsx! {
        document::Link { rel: "stylesheet", href: BUDGET_CSS }
        div {
            id: "budget_hero",
            if edit_budget_name() {
                input {
                    oninput: move |e| {
                        b_name.set(e.value().clone());
                    },
                    onkeydown: move |e| {
                        if e.code() == Code::Enter {
                            edit_budget_name.set(false);
                        }
                    },
                    type: "text",
                    value: "{b_name}",
                }
            } else {
                h4 {
                    onclick: move |_| {
                        edit_budget_name.set(true);
                    },
                    "{b_name}",
                }
            }
        }
    }
}

