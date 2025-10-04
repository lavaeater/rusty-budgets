use crate::budget::BudgetingTypeCard;
use crate::budget::BudgetingTypeOverviewView;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BankTransaction, Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn TransactionsView(budget_id: Uuid, transactions: Vec<BankTransaction>) -> Element {
    transactions
        .iter()
        .filter(|tx| tx.budget_item_id.is_none())
        .collect::<Vec<_>>()
        .sort_by_key(|tx| tx.date);
    let no_item = use_signal(|| transactions);

    rsx! {
        h1 { "Transactions" }
        h2 { {no_item().len().to_string()} }
        div { flex: "row",
            div {
                for transaction in no_item() {
                    p { {transaction.description} }
                    p { {transaction.amount.to_string()} }
                    p { {transaction.date.to_string()} }
                }
            }
        }

    }
}
