use crate::budget::budget_hero::BudgetState;
use crate::budget::workspace::BudgetTab;
use crate::budget::BudgetingTypeOverviewView;
use api::models::BudgetingType;
use api::view_models::BudgetViewModel;
use dioxus::prelude::*;

/// The landing snapshot: where you stand this period, and what needs attention.
///
/// Read-mostly by design — every call to action links to the tab that owns the
/// work rather than embedding the workflow here.
#[component]
pub fn OverviewTab(tab: Signal<BudgetTab>) -> Element {
    let budget = use_context::<BudgetState>().0();

    rsx! {
        div { class: "tab-panel",
            AttentionList { tab }

            div { class: "dashboard-cards",
                for overview in budget.overviews.clone() {
                    BudgetingTypeOverviewView {
                        budgeting_type: overview.budgeting_type,
                        overview,
                    }
                }
            }

            if budget.items.is_empty() {
                div { class: "empty-state",
                    h3 { "Inga budgetposter ännu" }
                    p { "Gruppera dina taggar i budgetposter för att komma igång." }
                    button {
                        class: "empty-state-action",
                        onclick: move |_| tab.set(BudgetTab::Plan),
                        "Gå till Budget →"
                    }
                }
            }
        }
    }
}

/// The "what should I do next" list, derived from the same counts that drive the
/// Att göra tab badge.
#[component]
fn AttentionList(tab: Signal<BudgetTab>) -> Element {
    let budget = use_context::<BudgetState>().0();
    let items = attention_items(&budget);

    if items.is_empty() {
        return rsx! {
            div { class: "attention-clear",
                span { class: "attention-clear-icon", "✓" }
                span { "Allt är hanterat för den här perioden." }
            }
        };
    }

    rsx! {
        div { class: "attention-list",
            h3 { class: "attention-list-title", "Behöver din uppmärksamhet" }
            for item in items {
                button {
                    class: "attention-row",
                    onclick: move |_| tab.set(item.target),
                    if let Some(count) = item.count {
                        span { class: "attention-row-count", {count.to_string()} }
                    }
                    span { class: "attention-row-label", {item.label.clone()} }
                    span { class: "attention-row-go", "→" }
                }
            }
        }
    }
}

#[derive(Clone)]
struct AttentionItem {
    /// `None` for rows whose subject is an amount rather than a tally.
    count: Option<usize>,
    label: String,
    target: BudgetTab,
}

fn attention_items(budget: &BudgetViewModel) -> Vec<AttentionItem> {
    let ready_to_assign = budget
        .overviews
        .iter()
        .find(|ov| ov.budgeting_type == BudgetingType::Income)
        .map(|ov| ov.remaining_budget);

    let mut items = Vec::new();
    if budget.untagged_transaction_count > 0 {
        items.push(AttentionItem {
            count: Some(budget.untagged_transaction_count),
            label: "transaktioner att tagga".to_string(),
            target: BudgetTab::Todo,
        });
    }
    if budget.potential_transfer_count > 0 {
        items.push(AttentionItem {
            count: Some(budget.potential_transfer_count),
            label: "möjliga interna överföringar".to_string(),
            target: BudgetTab::Todo,
        });
    }
    if !budget.to_connect.is_empty() {
        items.push(AttentionItem {
            count: Some(budget.to_connect.len()),
            label: "transaktioner att koppla".to_string(),
            target: BudgetTab::Todo,
        });
    }
    if let Some(rta) = ready_to_assign
        && rta.amount_in_cents() != 0
        && budget
            .items
            .iter()
            .any(|i| i.budgeting_type == BudgetingType::Income)
    {
        items.push(AttentionItem {
            count: None,
            label: if rta.amount_in_cents() < 0 {
                format!("{rta} överbudgeterat — flytta pengar mellan poster")
            } else {
                format!("{rta} kvar att fördela — ge varje krona ett syfte")
            },
            target: BudgetTab::Plan,
        });
    }
    items
}
