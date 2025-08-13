use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetItem() -> Element {
    rsx! {
        h1 { "Blog" }
    }
}

#[component]
pub fn NewBudgetItem(budget_id: Uuid, item_type: String) -> Element {
    let nav = navigator();
    let mut name = use_signal(|| "Budgetkategori".to_string());
    let mut first_item = use_signal(|| "Första post".to_string());
    let mut amount = use_signal(|| 0.0);
    let mut expected_at = use_signal(|| "2025-08-25".to_string());
    rsx! {
            document::Link { rel: "stylesheet", href: BUDGET_CSS}

            div {
                input {
                    oninput: move |e| {
                        *name.write() = e.value().clone();
                    },
                    r#type: "text",
                    value: "{name.read()}"
                }
            }
            div {
                input {
                    oninput: move |e| {
                        *first_item.write() = e.value().clone();
                    },
                    r#type: "text",
                    value: "{first_item.read()}"
                }
                input {
                    oninput: move |e| {
                        *amount.write() = e.value().parse().unwrap();
                    },
                    r#type: "number",
                    value: "{amount.read()}"
                }
            }
            div {
                input { oninput: move |e| {
                        *expected_at.write() = e.value().clone();
                    },
                    r#type: "date",
                    value: "{expected_at.read()}"
                }
            }
            div {
                button {
                    onclick: move |_| {
                    spawn(async move {
                    match api::add_budget_item(
                        budget_id,
                        name.read().clone(),
                        first_item.read().clone(),
                        amount.read().clone(),
                    ).await {
                        Ok(_) => {
                            tracing::info!("Success");
                            nav.replace("/");
                        }
                        Err(e) => {
                            tracing::error!("Failed to save budget: {}", e);
                        }
                    };
                    });
                },
                    "Spara"
                }
            }
    }
}
