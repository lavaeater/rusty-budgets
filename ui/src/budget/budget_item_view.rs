use crate::Button;
use api::models::*;
use dioxus::fullstack::server_fn::serde::{Deserialize, Serialize};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType) -> Element {
    let budget = use_context::<Signal<Option<Budget>>>();
    let mut expanded = use_signal(|| false);
    if expanded() {
        let mut transactions: Signal<Vec<BankTransaction>> = use_signal(|| vec![]);
        match budget() {
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

                    // Left side: name
                    div { class: "font-medium", "EXPANDED ! {item.name}" }

                    // Right side: actual / budgeted
                    div { class: "text-gray-700",
                        "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    }
                    div { flex_direction: "row",
                        for transaction in transactions() {
                            div { flex_direction: "column",
                                p { class: "text-gray-700", "{transaction.description}" }
                                p { class: "text-gray-700", "{transaction.amount.to_string()}" }
                            }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            button { style: "none",
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
}
