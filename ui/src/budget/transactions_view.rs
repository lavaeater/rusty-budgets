use crate::budget::{ItemSelector, NewBudgetItem};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use api::view_models::{BudgetItemViewModel, BudgetViewModel};
use crate::{Button, PopoverContent, PopoverRoot, PopoverTrigger};

#[component]
pub fn TransactionsView(ignored: bool) -> Element {
    let mut budget_signal = use_context::<Signal<Option<BudgetViewModel>>>();
    
    match budget_signal() {
        Some(budget) => {
            let transactions = if ignored {
                budget.ignored_transactions.clone()
            } else {
                budget.to_connect.clone()
            };
            rsx! {
                div { class: "transactions-view-a",
                    h2 { class: "transactions-title",
                        if ignored {
                            "Ignorerade transaktioner "
                        } else {
                            "Ohanterade transaktioner "
                        }
                        span { class: "transaction-count", "({transactions.len()})" }
                    }
                    div { class: "transactions-list",
                        for tx in transactions {
                            div { class: "transaction-card",
                                div { class: "transaction-info",
                                    div { class: "transaction-description",
                                        strong { {tx.description.to_string()} }
                                    }
                                    div { class: "transaction-meta",
                                        span { class: "transaction-date",
                                            {tx.date.format("%Y-%m-%d").to_string()}
                                        }
                                        span { class: if tx.amount.is_pos() { "transaction-amount positive" } else { "transaction-amount negative" },
                                            {tx.amount.to_string()}
                                        }
                                    }
                                }
                                div { class: "transaction-actions",
                                    div { class: "action-group",
                                        ItemSelector {
                                            items: budget.items.clone(),
                                            on_change: move |e: Option<BudgetItemViewModel>| async move {
                                                if let Some(item) = e {
                                                    info!("Lets connect transaction {} to item {}", tx.tx_id, item.item_id);
                                                    match connect_transaction(
                                                            budget.id,
                                                            tx.tx_id,
                                                            item.actual_id,
                                                            item.item_id,
                                                            budget.period_id,
                                                        )
                                                        .await {
                                                            Ok(bv) => {
                                                                info!("Connected transaction {} to item {}", tx.tx_id, item.item_id);
                                                                budget_signal.set(Some(bv));
                                                            }
                                                            Err(e) => {
                                                                error!("Failed to connect transaction {} to item {}: {}", tx.tx_id, item.item_id, e);
                                                            }
                                                        } 
                                                }
                                            },
                                        }
                                    }
                                    div { class: "action-group",
                                        if tx.amount.is_pos() {
                                            NewBudgetItemPopover {
                                                budgeting_type: BudgetingType::Income,
                                                tx_id: Some(tx.tx_id),
                                            }
                                        } else {
                                            NewBudgetItemPopover {
                                                budgeting_type: BudgetingType::Expense,
                                                tx_id: Some(tx.tx_id),
                                            }
                                            NewBudgetItemPopover {
                                                budgeting_type: BudgetingType::Savings,
                                                tx_id: Some(tx.tx_id),
                                            }
                                        }
                                    }
                                    Button {
                                        r#type: "button",
                                        "data-style": "destructive",
                                        onclick: move |_| async move {
                                            info!("Ignoring: {} in {}", tx.tx_id, budget.period_id);
                                            if let Ok(bv) = api::ignore_transaction(budget.id, tx.tx_id, budget.period_id)
                                                .await
                                            {
                                                budget_signal.set(Some(bv));
                                            }
                                        },
                                        "Ignorera"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { "No budget found" }
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
