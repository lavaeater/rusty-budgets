use crate::{Button, Input};
use api::models::*;
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;
use api::connect_transaction;
use crate::budget::ItemSelector;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let mut expanded = use_signal(|| false);
    let mut edit_item = use_signal(|| false);
    let mut item_name = use_signal(|| item.name.clone());
    let budget = budget_signal().unwrap();
    let items = budget.list_all_items();
    let budget_id = budget.id;
    
    // State for selected transaction IDs and the target item for moving
    let mut selected_transactions = use_signal(HashSet::<Uuid>::new);
    let mut show_move_selector = use_signal(|| false);
    
    if expanded() {
        let transactions = use_signal(|| {
            budget_signal()
                .as_ref()
                .map(|b| b.
                    list_transactions_for_item(&item.id, true)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<BankTransaction>>())
                .unwrap_or_default()
        });
        
        if edit_item() {
            rsx! {
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
                                    budget_id,
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
            }
        } else {
            rsx! {
                div {
                    class: "flex flex-col p-2 border-b border-gray-200 text-sm",
                    // Header with item name and amount
                    div {
                        class: "flex justify-between items-center",
                        onclick: move |_| { edit_item.set(!edit_item()) },
                        div { class: "font-large", "{item_name()}" }
                        div { class: "text-gray-700",
                            "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                        }
                    }
                    // Transaction list with checkboxes
                    // div { class: "mt-2",
                    // 
                    //     
                    // }
                        for transaction in transactions() {
                            let tx_id = transaction.id;
                            let is_selected = selected_transactions().contains(&tx_id);
                        
                            div {
                                class: "flex items-center p-1 hover:bg-gray-50 rounded",
                                input {
                                    r#type: "checkbox",
                                    checked: is_selected,
                                    onchange: move |_| {
                                        let mut selected = selected_transactions();
                                        if is_selected {
                                            selected.remove(&tx_id);
                                        } else {
                                            selected.insert(tx_id);
                                        }
                                        selected_transactions.set(selected);
                                    },
                                    class: "mr-2 h-4 w-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500"
                                }
                                p {
                                    class: "flex-1 text-gray-700",
                                    "{transaction.description}"
                                }
                                p {
                                    class: "text-gray-700 w-24 text-right",
                                    "{transaction.amount.to_string()}"
                                }
                            }
                        }
                    // Action buttons (only show when transactions are selected)
                    if !selected_transactions().is_empty() {
                        div { class: "mt-2 flex items-center gap-2",
                            Button {
                                r#type: "button",
                                "data-style": "secondary",
                                onclick: move |_| {
                                    selected_transactions.set(HashSet::new());
                                },
                                "Avmarkera alla"
                            }

                            if !show_move_selector() {
                                Button {
                                    r#type: "button",
                                    "data-style": "primary",
                                    onclick: move |_| {
                                        show_move_selector.set(true);
                                    },
                                    "Flytta markerade"
                                }
                            } else {
                                div { class: "flex-1 flex items-center gap-2",
                                    span { "Flytta till:" }
                                    ItemSelector {
                                        items: items
                                            .iter()
                                            .filter(|i| i.id != item.id) // Don't show current item
                                            .cloned()
                                            .collect(),
                                        on_change: move |target_item: Option<BudgetItem>| async move {
                                            if let Some(target_item) = target_item {
                                                let mut success = true;
                                                let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                                for tx_id in selected_ids {
                                                    if let Err(_) = api::connect_transaction(budget_id, tx_id, target_item.id).await {
                                                        success = false;
                                                        break;
                                                    }
                                                }

                                                if success {
                                                    // Refresh the budget data
                                                    if let Ok(updated_budget) = api::get_budget(Some(budget_id)).await {
                                                        budget_signal.set(updated_budget);
                                                    }
                                                    selected_transactions.set(HashSet::new());
                                                    show_move_selector.set(false);
                                                }
                                            } else {
                                                show_move_selector.set(false);
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
                div { class: "font-large", "{item_name()}" }

                // Right side: actual / budgeted
                div { class: "text-gray-700",
                    "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                }
            }
        }
    }
}
