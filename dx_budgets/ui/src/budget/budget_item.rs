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
pub fn NewBudgetItem(budget_id: Uuid) -> Element {
    let mut text = use_signal(|| "What is it?".to_string());
    let mut amount = use_signal(|| 0.0);
    let mut expected_at = use_signal(|| "2025-08-25".to_string());
    rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS}

                div {
                    input {
                        oninput: move |e| {
                            *text.write() = e.value().clone();
                        },
                        r#type: "text",
                        value: "{text.read()}"
                    }
                }
                div {
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
                            text.read().clone(),
                            amount.read().clone(),
                            expected_at.read().clone(),
                        ).await {  
                            Ok(_) => {                                                
                                tracing::info!("Success");
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
