use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use crate::budget::ItemSelector;
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;

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
                    items,
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
    
    
    match transactions.first() {
        Some(tx) => {
            let tx = tx.clone();
            
        }
            None => {
                rsx! {
                    h1 { "Transactions" }
                    h2 { "No transactions" }
                }
            }
    }
}
