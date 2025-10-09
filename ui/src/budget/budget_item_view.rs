use crate::{Button, Input};
use api::models::*;
use dioxus::fullstack::server_fn::serde::{Deserialize, Serialize};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let mut expanded = use_signal(|| false);
    let mut edit_item = use_signal(||false);
    let mut item_name = use_signal(|| item.name.clone());
    if expanded() {
        let mut transactions: Signal<Vec<BankTransaction>> = use_signal(|| vec![]);
        match budget_signal() {
            None => {}
            Some(budget) => {
                transactions.set(
                    budget
                        .list_transactions_for_item(&item.id, true)
                        .into_iter()
                        .cloned()
                        .collect(),
                );
            }
        }

        rsx! {
            button { style: "none",
                div {
                    class: "flex justify-between items-center p-2 border-b border-gray-200 text-sm",
                    onclick: move |_| { expanded.set(!expanded()) },
                    if edit_item() {
                        Input {
                            value: item_name(),
                            oninput: move |e: FormEvent| { item_name.set(e.value()) },
                        }
                        Button {
                            r#type: "button",
                            "data-style": "primary",
                            onclick: move |_| async move {
                                if let Ok(updated_budget) = api::modify_item(
                                        budget_signal().unwrap().id,
                                        item.id,
                                        Some(item_name()),
                                        None,
                                        None,
                                    )
                                    .await
                                {
                                    budget_signal.set(Some(updated_budget));
                                }
                            },
                            "Uppdatera"
                        }
                    } else {
                        div {
                            class: "font-large",
                            onclick: move |_| { edit_item.set(!edit_item()) },
                        }
                        "{item_name()}"
                    }


                    // Right side: actual / budgeted
                    div { class: "text-gray-700",
                        "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    }
                    for transaction in transactions() {
                        div { display: "flex", flex_direction: "row",
                            p { class: "text-gray-700", "{transaction.description}" }
                            p { class: "text-gray-700", "{transaction.amount.to_string()}" }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "flex justify-between items-center p-2 border-b border-gray-200 text-sm",
                onclick: move |_| { expanded.set(!expanded()) },

                // Left side: name
                div { class: "font-large", "{item.name}" }

                // Right side: actual / budgeted
                div { class: "text-gray-700",
                    "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                }
            }
        }
    }
}
