use crate::budget::{ItemSelector, NewBudgetItem};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::{Button, PopoverContent, PopoverRoot, PopoverTrigger};

#[component]
pub fn TransactionsView(budget_id: Signal<Uuid>, to_connect: Signal<Vec<BankTransaction>>, ignored: Signal<Vec<BankTransaction>>, items: Signal<Vec<BudgetItem>>) -> Element {
    rsx! {
        div { class: "transactions-view-a",
            h2 { class: "transactions-title",
                "Ohanterade transaktioner "
                span { class: "transaction-count", "({to_connect().len()})" }
            }
            div { class: "transactions-list",
                for tx in to_connect() {
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
                                    items: items,
                                    on_change: move |e: Option<BudgetItem>| async move {
                                        if let Some(item) = e {
                                            
                                            
                                            if let Ok(()) = connect_transaction(budget_id(), tx.id, item.id).await {
                                                to_connect.set(to_connect().into_iter().filter(|t| t.id != tx.id).collect());
                                            }
                                        }
                                    },
                                }
                            }
                            div { class: "action-group",
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
                            Button {
                                r#type: "button",
                                "data-style": "destructive",
                                onclick: move |_| async move {
                                    if let Ok(()) = api::ignore_transaction(budget_id(), tx.id).await {
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
