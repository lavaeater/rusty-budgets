use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>, items: Vec<BudgetItem>) -> Element {
    match transactions.first() {
        Some(tx) => {
            rsx! {
                h1 { "Transactions" }
                h2 { {transactions.len().to_string()} }
                div {
                    div { display: "flex", flex_direction: "row", gap: "1rem",
                        p { {tx.description.to_string()} }
                        p { {tx.amount.to_string()} }
                        p { {tx.date.format("%Y-%m-%d").to_string()} }
                    }
                }
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
