use crate::Button;
use api::models::*;
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;
use api::ignore_transaction;
use crate::budget::ItemSelector;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType, is_over_budget: bool) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let budget_period_id = use_context::<Signal<Option<BudgetPeriodId>>>();
    let mut expanded = use_signal(|| false);
    // let mut edit_item = use_signal(|| false);
    let item_name = use_signal(|| item.name.clone());
    let budget = budget_signal().unwrap();
    let items = budget.list_all_items();
    let budget_id = budget.id;
    
    // State for selected transaction IDs and the target item for moving
    let mut selected_transactions = use_signal(HashSet::<Uuid>::new);
    let mut show_move_selector = use_signal(|| false);
    
    if expanded() {
        let transactions = 
            budget_signal()
                .as_ref()
                .map(|b| b.
                    list_transactions_for_item(&item.id, true)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<BankTransaction>>())
                .unwrap_or_default();
            rsx! {
                div { class: "flex flex-col p-2 border-b border-gray-200 text-sm",
                    // Header with item name and amount
                    div { class: "flex justify-between items-center",
                        // onclick: move |_| { edit_item.set(!edit_item()) },
                        div { class: "font-large", "{item_name()}" }
                        div { class: "text-gray-700",
                            "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                        }
                    }
                    // Transaction list with checkboxes
                    div { class: "mt-2",
                        {
                            transactions
                                .iter()
                                .map(|transaction| {
                                    let tx_id = transaction.id;
                                    let is_selected = selected_transactions().contains(&tx_id);
                                    rsx! {
                                        div { class: "flex items-center p-1 hover:bg-gray-50 rounded",
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
                                            }
                                            {transaction.description.clone()}
                                            {transaction.amount.to_string()}
                                        }
                                    }
                                })
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
                            Button {
                                r#type: "button",
                                "data-style": "destructive",
                                onclick: move |_| async move {
                                    let mut success = true;
                                    let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                    for tx_id in selected_ids {

                                        // Refresh the budget data
                                        if let Err(_) = ignore_transaction(budget_id, tx_id).await {
                                            success = false;
                                            break;
                                        }
                                    }
                                    if success {
                                        if let Ok(updated_budget) = api::get_budget(Some(budget_id)).await {
                                            budget_signal.set(updated_budget);
                                        }
                                        selected_transactions.set(HashSet::new());
                                        show_move_selector.set(false);
                                    }
                                },
                                "Ignorera alla"
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
                                        items: items.iter().filter(|i| i.id != item.id).cloned().collect(),
                                        on_change: move |target_item: Option<BudgetItem>| async move {
                                            if let Some(target_item) = target_item {
                                                let mut success = true;
                                                let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                                for tx_id in selected_ids {
                                                    if let Err(_) = api::connect_transaction(

                                                            // Refresh the budget data
                                                            budget_id,
                                                            tx_id,
                                                            target_item.id,
                                                        )
                                                        .await
                                                    {
                                                        success = false;
                                                        break;
                                                    }
                                                }
                                                if success {
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
        // }
    } else {
        rsx! {
            div { class: format!("budget-item {}", if is_over_budget { "over-budget" } else { "" }),
                // Left side: name
                div {
                    class: "font-large",
                    onclick: move |_| { expanded.set(!expanded()) },
                    "{item_name()}"
                }
                // Right side: actual / budgeted
                div { class: "text-gray-700",
                    "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                }
                if is_over_budget {
                    span { class: "over-budget-indicator", "Over Budget" }
                    // Auto-adjust button if there's available income
                    {
                        let shortage = item.actual_amount - item.budgeted_amount;
                        let can_auto_adjust = budget_signal()
                            .as_ref()
                            .and_then(|b| b.get_budgeting_overview(&BudgetingType::Income))
                            .map(|o| o.remaining_budget >= shortage)
                            .unwrap_or(false);

                        if can_auto_adjust {
                            rsx! {
                                button {
                                    class: "auto-adjust-button",
                                    onclick: move |_| {
                                        let item_id = item.id;
                                        let shortage = shortage;
                                        spawn(async move {
                                            match api::adjust_item_funds(budget_id, item_id, shortage, budget_period_id()).await {
                                                Ok(updated_budget) => {
                                                    budget_signal.set(Some(updated_budget));
                                                }
                                                Err(e) => {
                                                    dioxus::logger::tracing::error!(
                                                        "Failed to adjust item funds: {}", e
                                                    );
                                                }
                                            }
                                        });
                                    },
                                    "Auto-Adjust (+{shortage})"
                                }
                            }
                        } else {
                            rsx! {}
                        }
                    }
                }
            }
        }
    }
}
