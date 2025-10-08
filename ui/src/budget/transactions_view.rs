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
                if tx.amount.is_pos() {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "1rem",
                        NewBudgetItemPopover { budgeting_type: BudgetingType::Income }
                    }
                } else {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: "1rem",
                        NewBudgetItemPopover { budgeting_type: BudgetingType::Expense }
                        NewBudgetItemPopover { budgeting_type: BudgetingType::Savings }
                    }
                }
            
            }
        }

    }
}

#[component]
pub fn NewBudgetItemPopover(budgeting_type: BudgetingType) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        PopoverRoot { open: open(), on_open_change: move |v| open.set(v),
            PopoverTrigger { "Ny {{budgeting_type.to_string()}}" }
            PopoverContent { gap: "0.25rem",
                NewBudgetItem { budgeting_type, close_signal: Some(open) }
            }
        }
    }
}
