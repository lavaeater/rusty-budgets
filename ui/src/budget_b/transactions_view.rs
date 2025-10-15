use crate::budget_b::{ItemSelector, NewBudgetItem};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::{Button, PopoverContent, PopoverRoot, PopoverTrigger};

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>, items: Vec<BudgetItem>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let mut selected_tx = use_signal(|| None::<usize>);
    
    rsx! {
        div { class: "transactions-view-b",
            for (idx, tx) in transactions.into_iter().enumerate() {
                div { 
                    class: if selected_tx() == Some(idx) { "transaction-item-b selected" } else { "transaction-item-b" },
                    onclick: move |_| {
                        selected_tx.set(if selected_tx() == Some(idx) { None } else { Some(idx) });
                    },
                    
                    div { class: "transaction-header-b",
                        div { class: "transaction-desc-b",
                            strong { {tx.description.to_string()} }
                            div { class: "transaction-date-b", {tx.date.format("%Y-%m-%d").to_string()} }
                        }
                        span { 
                            class: if tx.amount.is_pos() { "transaction-amt-b positive" } else { "transaction-amt-b negative" },
                            {tx.amount.to_string()}
                        }
                    }
                    
                    if selected_tx() == Some(idx) {
                        div { class: "transaction-actions-b",
                            div { class: "action-row",
                                label { class: "action-label", "Koppla till:" }
                                ItemSelector {
                                    items: items.clone(),
                                    on_change: move |e: Option<BudgetItem>| async move {
                                        if let Some(item) = e {
                                            if let Ok(budget) = connect_transaction(budget_id, tx.id, item.id).await {
                                                budget_signal.set(Some(budget));
                                                selected_tx.set(None);
                                            }
                                        }
                                    },
                                }
                            }
                            div { class: "action-row",
                                label { class: "action-label", "Eller skapa ny:" }
                                div { class: "create-buttons",
                                    if tx.amount.is_pos() {
                                        NewBudgetItemPopover {
                                            budgeting_type: BudgetingType::Income,
                                            tx_id: Some(tx.id),
                                        }
                                    } else {
                                        NewBudgetItemPopover {
                                            budgeting_type: BudgetingType::Expense,
                                            tx_id: Some(tx.id),
                                        }
                                        NewBudgetItemPopover {
                                            budgeting_type: BudgetingType::Savings,
                                            tx_id: Some(tx.id),
                                        }
                                    }
                                }
                            }
                            div { class: "action-row",
                                Button {
                                    r#type: "button",
                                    "data-style": "destructive",
                                    onclick: move |_| async move {
                                        if let Ok(updated_budget) = api::ignore_transaction(budget_id, tx.id).await {
                                            budget_signal.set(Some(updated_budget));
                                            selected_tx.set(None);
                                        }
                                    },
                                    "Ignorera transaktion"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn NewBudgetItemPopover(budgeting_type: BudgetingType, tx_id: Option<Uuid>) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        PopoverRoot { open: open(), on_open_change: move |v| open.set(v),
            PopoverTrigger { {budgeting_type.to_string()} }
            PopoverContent { gap: "0.25rem",
                NewBudgetItem { budgeting_type, tx_id, close_signal: Some(open) }
            }
        }
    }
}
