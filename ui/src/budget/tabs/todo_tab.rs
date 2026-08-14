use crate::budget::budget_hero::BudgetState;
use crate::budget::{TagTransactionsView, TransactionsView, TransferPairsView};
use dioxus::prelude::*;

/// The work queue: everything waiting on a decision from you.
///
/// Previously these sections appeared and disappeared from the main page as
/// counts changed; here they are always in the same place, each with its own
/// heading and count.
#[component]
pub fn TodoTab() -> Element {
    let budget = use_context::<BudgetState>().0();

    let nothing_to_do = budget.untagged_transaction_count == 0
        && budget.potential_transfer_count == 0
        && budget.to_connect.is_empty();

    if nothing_to_do {
        return rsx! {
            div { class: "tab-panel",
                div { class: "transactions-section-minimal",
                    p { class: "success-message", "✓ Inget att göra — alla transaktioner är hanterade!" }
                }
            }
        };
    }

    rsx! {
        div { class: "tab-panel",
            if budget.untagged_transaction_count > 0 {
                section { class: "todo-section",
                    h3 { class: "todo-section-title",
                        "{budget.untagged_transaction_count} transaktioner att tagga"
                    }
                    TagTransactionsView {}
                }
            }
            if budget.potential_transfer_count > 0 {
                section { class: "todo-section",
                    h3 { class: "todo-section-title",
                        "{budget.potential_transfer_count} möjliga interna överföringar"
                    }
                    TransferPairsView {}
                }
            }
            if !budget.to_connect.is_empty() {
                section { class: "todo-section",
                    h3 { class: "todo-section-title",
                        "{budget.to_connect.len()} transaktioner att koppla"
                    }
                    TransactionsView { ignored: false }
                }
            }
        }
    }
}
