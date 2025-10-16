use crate::{Button, Input};
use api::models::*;
use dioxus::fullstack::server_fn::serde::{Deserialize, Serialize};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::budget::ItemSelector;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let mut expanded = use_signal(|| false);
    let mut edit_item = use_signal(||false);
    let mut item_name = use_signal(|| item.name.clone());
    let items = budget_signal().unwrap().list_all_items();
    let budget_id = budget_signal().unwrap().id;
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
            if edit_item() {
                div { class: "flex justify-between items-center p-2 border-b border-gray-200 text-sm",
                    Input {
                        id: "item_name",
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
                            edit_item.set(false);
                        },
                        "Uppdatera"
                    }
                }
            } else {
                div { class: "flex justify-between items-center p-2 border-b border-gray-200 text-sm",
                    div {
                        class: "font-large",
                        onclick: move |_| { edit_item.set(!edit_item()) },

                        "{item_name()}"
                    }

                    div {
// onclick: move |_| { expanded.set(!expanded()) },                        // Right side: actual / budgeted
                        div { class: "text-gray-700",
                            "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                        }
                        for transaction in transactions() {
                            div { display: "flex", flex_direction: "row",
                                p { class: "text-gray-700", "{transaction.description}" }
                                p { class: "text-gray-700", "{transaction.amount.to_string()}" }
                                div { class: "action-group",
                                    ItemSelector {
                                        items: items.clone(),
                                        on_change: move |e: Option<BudgetItem>| async move {
                                            if let Some(item) = e {
                                                if let Ok(budget) = connect_transaction(budget_id, transaction.id, item.id).await {
                                                    budget_signal.set(Some(budget));
                                                }
                                            }
                                        },
                                    }
                                }
                            }
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
