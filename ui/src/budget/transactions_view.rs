use crate::budget::BudgetingTypeCard;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget::BudgetingTypeOverviewView;

#[component]
pub fn TransactionsView(
    budget_id: Uuid,
    transactions: Vec<BankTransaction>,
) -> Element {
    rsx! {
        h1 { "Transactions" }
        h2 { { transactions.len().to_string() }}
        div { flex: "row",
            div {
                for transaction in transactions {
                    p { {transaction.description} }
                    p { {transaction.amount.to_string()} }
                    p { {transaction.date.to_string()} }
                }
            }
        }

    }
}