use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use crate::budget::ItemSelector;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>, items: Vec<BudgetItem>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    match transactions.first() {
        Some(tx) => {
            rsx! {
                h1 { "Transactions" }
                h2 { {transactions.len().to_string()} }
                div { display: "flex", flex_direction: "row", gap: "1rem",
                    p { {tx.description.to_string()} }
                    p { {tx.amount.to_string()} }
                    p { {tx.date.format("%Y-%m-%d").to_string()} }
                }
                ItemSelector { items }

            }
        }
            None => {
                rsx! {
                    h1 { "Transactions" }
                    h2 { "No transactions" }
                }
            }
    }
}
