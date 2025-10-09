use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use crate::budget::ItemSelector;
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::{Button, NewBudgetItem, PopoverContent, PopoverRoot, PopoverTrigger};

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>, items: Vec<BudgetItem>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    rsx! {
        h1 { "Transactions" }
        h2 { {transactions.len().to_string()} }
        for tx in transactions {
            div {
                display: "flex",
                flex_direction: "row",
                gap: "1rem",
                height: "3rem",
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
                Button {
                    r#type: "button",
                    "data-style": "destructive",
                    onclick: move |_| async move {
                        if let Ok(updated_budget) = api::ignore_transaction(budget_id, tx.id).await {
                            budget_signal.set(Some(updated_budget));
                        }
                    },
                    "Ignorera"
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
