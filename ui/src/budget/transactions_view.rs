use crate::budget::{ItemSelector, NewBudgetItem};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use api::view_models::BudgetViewModel;
use api::view_models::BudgetItemViewModel;
use crate::{Button, PopoverContent, PopoverRoot, PopoverTrigger};
use crate::budget::budget_hero::BudgetState;

#[component]
pub fn TransactionsView(ignored: bool) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    
    let transactions = if ignored {
                budget_signal().ignored_transactions.clone()
            } else {
                budget_signal().to_connect.clone()
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
                            div { class: "transaction-card", key: "{tx.tx_id}",
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
                                            items: budget_signal().items.clone(),
                                            on_change: move |e: Option<BudgetItemViewModel>| async move {
                                                if let Some(item) = e {
                                                    info!("Lets connect transaction {} to item {}", tx.tx_id, item.item_id);
                                                    match connect_transaction(
                                                            budget_signal().id,
                                                            tx.tx_id,
                                                            item.actual_id,
                                                            item.item_id,
                                                            budget_signal().period_id,
                                                        )
                                                        .await
                                                    {
                                                        Ok(bv) => {
                                                            info!("Connected transaction {} to item {}", tx.tx_id, item.item_id);
                                                            consume_context::<BudgetState>().0.set(bv);
                                                        }
                                                        Err(e) => {
                                                            error!(
                                                                "Failed to connect transaction {} to item {}: {}", tx.tx_id, item
                                                                .item_id, e
                                                            );
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
                                            info!("Ignoring: {} in {}", tx.tx_id, budget_signal().period_id);
                                            if let Ok(bv) = api::ignore_transaction(
                                                    budget_signal().id,
                                                    tx.tx_id,
                                                    budget_signal().period_id,
                                                )
                                                .await
                                            {
                                                consume_context::<BudgetState>().0.set(bv);
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
