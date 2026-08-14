use crate::budget::budget_hero::BudgetState;
use crate::budget::{RetagTransactionsView, TransactionsView};
use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TransactionFilter {
    #[default]
    Tagged,
    Unconnected,
    Ignored,
}

impl TransactionFilter {
    fn label(self) -> &'static str {
        match self {
            TransactionFilter::Tagged => "Taggade",
            TransactionFilter::Unconnected => "Att koppla",
            TransactionFilter::Ignored => "Ignorerade",
        }
    }
}

/// Every transaction in the period, behind one filter control.
///
/// Consolidates what used to be two separate `TransactionsView` sections plus a
/// collapsed "Taggade transaktioner" block.
#[component]
pub fn TransactionsTab() -> Element {
    let budget = use_context::<BudgetState>().0();
    let mut filter = use_signal(TransactionFilter::default);

    let counts = [
        (TransactionFilter::Tagged, None),
        (
            TransactionFilter::Unconnected,
            Some(budget.to_connect.len()),
        ),
        (
            TransactionFilter::Ignored,
            Some(budget.ignored_transactions.len()),
        ),
    ];

    rsx! {
        div { class: "tab-panel",
            div { class: "filter-bar",
                for (candidate , count) in counts {
                    button {
                        class: if filter() == candidate { "filter-chip filter-chip-active" } else { "filter-chip" },
                        onclick: move |_| filter.set(candidate),
                        {candidate.label()}
                        if let Some(count) = count {
                            span { class: "filter-chip-count", {count.to_string()} }
                        }
                    }
                }
            }

            match filter() {
                TransactionFilter::Tagged => rsx! {
                    RetagTransactionsView {}
                },
                TransactionFilter::Unconnected => rsx! {
                    if budget.to_connect.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Alla transaktioner är kopplade!" }
                        }
                    } else {
                        TransactionsView { ignored: false }
                    }
                },
                TransactionFilter::Ignored => rsx! {
                    if budget.ignored_transactions.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Inga ignorerade transaktioner!" }
                        }
                    } else {
                        TransactionsView { ignored: true }
                    }
                },
            }
        }
    }
}
