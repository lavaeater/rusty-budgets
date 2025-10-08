use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use crate::budget::ItemSelector;
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::{Button, PopoverContent, PopoverRoot, PopoverTrigger};

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>, items: Vec<BudgetItem>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    rsx! {
        h1 { "Transactions" }
        h2 { {transactions.len().to_string()} }
        for tx in transactions {

            div { display: "flex", flex_direction: "row", gap: "1rem",
                p { {tx.description.to_string()} }
                p { {tx.amount.to_string()} }
                p { {tx.date.format("%Y-%m-%d").to_string()} }
                ItemSelector {
                    items: items.clone(),
                    on_change: move |e: Option<BudgetItem>| async move {
                        if let Some(item) = e {
                            if let Ok(budget) = connect_transaction(budget_id, tx.id, item.id).await {
                                budget_signal.set(Some(budget));
                            }
                        }
                    },
                }
            }
        }

    }
}

#[component]
pub fn NewBudgetItemPopover() -> Element {
    let mut open = use_signal(|| false);
    let mut confirmed = use_signal(|| false);
    rsx! {
        PopoverRoot { open: open(), on_open_change: move |v| open.set(v),
            PopoverTrigger { "Ny budgetpost" }
            PopoverContent { gap: "0.25rem",
                h3 {
                    padding_top: "0.25rem",
                    padding_bottom: "0.25rem",
                    width: "100%",
                    text_align: "center",
                    margin: 0,
                    "Delete Item?"
                }
                Button {
                    r#type: "button",
                    "data-style": "outline",
                    onclick: move |_| {
                        open.set(false);
                        confirmed.set(true);
                    },
                    "Confirm"
                }
                Button {
                    r#type: "button",
                    "data-style": "outline",
                    onclick: move |_| {
                        open.set(false);
                    },
                    "Cancel"
                }
            }
        }
        if confirmed() {
            p { style: "color: var(--contrast-error-color); margin-top: 16px; font-weight: 600;",
                "Item deleted!"
            }
        }
    }
}
