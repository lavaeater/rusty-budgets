//! Layer 3 — component render tests (see docs/testing.md).
//!
//! These render real Dioxus components to static HTML on the native target
//! using `dioxus-ssr` purely as a test harness (there is no SSR in production
//! for the UI crate). A single `rebuild_in_place` pass renders *initial state
//! only* — effects, JS (`document::eval`) and event handlers do not run, so
//! these assert render correctness / prop→DOM wiring, not interaction.

use api::models::{BudgetingType, Currency, Money};
use api::view_models::{
    BudgetItemStatus, BudgetItemViewModel, BudgetViewModel, BudgetingTypeOverview,
};
use dioxus::prelude::*;
use ui::budget::{BudgetItemStatusView, BudgetState, BudgetingTypeOverviewView};

fn overview(
    budgeted: i64,
    actual: i64,
    remaining: i64,
    is_ok: bool,
) -> BudgetingTypeOverview {
    BudgetingTypeOverview {
        budgeting_type: BudgetingType::Expense,
        budgeted_amount: Money::new_dollars(budgeted, Currency::SEK),
        actual_amount: Money::new_dollars(actual, Currency::SEK),
        remaining_budget: Money::new_dollars(remaining, Currency::SEK),
        is_ok,
    }
}

/// Render a single component tree to HTML.
fn render(el: Element) -> String {
    dioxus_ssr::render_element(el)
}

#[test]
fn overview_card_shows_type_and_amounts() {
    let html = render(rsx! {
        BudgetingTypeOverviewView {
            budgeting_type: BudgetingType::Expense,
            overview: overview(1000, 400, 600, true),
        }
    });

    // The heading is the budgeting-type label.
    assert!(html.contains(&BudgetingType::Expense.to_string()));
    // The three money figures reach the DOM.
    assert!(html.contains(&Money::new_dollars(1000, Currency::SEK).to_string()), "budgeted");
    assert!(html.contains(&Money::new_dollars(400, Currency::SEK).to_string()), "actual");
    assert!(html.contains(&Money::new_dollars(600, Currency::SEK).to_string()), "remaining");
    // The Swedish stat labels are present.
    assert!(html.contains("Budgeterat"));
    assert!(html.contains("Faktiskt"));
    assert!(html.contains("Återstår"));
}

#[test]
fn overview_bar_width_is_actual_over_budgeted() {
    // 400 / 1000 = 40%.
    let html = render(rsx! {
        BudgetingTypeOverviewView {
            budgeting_type: BudgetingType::Expense,
            overview: overview(1000, 400, 600, true),
        }
    });
    assert!(html.contains("width: 40%"), "expected 40% bar, got: {html}");
    // Not over budget -> no `over` modifier on the fill.
    assert!(html.contains("overview-bar-fill"));
    assert!(!html.contains("overview-bar-fill over"));
}

#[test]
fn overview_bar_caps_at_100_and_marks_over() {
    // actual (1500) > budgeted (1000): bar capped at 100% and marked `over`.
    let html = render(rsx! {
        BudgetingTypeOverviewView {
            budgeting_type: BudgetingType::Expense,
            overview: overview(1000, 1500, -500, false),
        }
    });
    assert!(html.contains("width: 100%"), "bar should cap at 100%");
    assert!(html.contains("overview-bar-fill over"), "over-budget fill modifier");
    // is_ok = false -> the card carries the over-budget class and warning heading.
    assert!(html.contains("over-budget"));
    assert!(html.contains("warning"));
}

#[test]
fn overview_bar_zero_when_nothing_budgeted() {
    let html = render(rsx! {
        BudgetingTypeOverviewView {
            budgeting_type: BudgetingType::Income,
            overview: overview(0, 0, 0, true),
        }
    });
    assert!(html.contains("width: 0%"), "no budget -> 0% bar");
}

// ---------------------------------------------------------------------------
// Context-provider pattern: views that read `use_context::<BudgetState>()`.
//
// A tiny harness component installs the shared BudgetState signal, then renders
// the component under test. This is the reusable pattern for every workflow
// view (tag_transactions_view, create_budget_items_view, …) — they all consume
// the same context.
// ---------------------------------------------------------------------------

#[component]
fn StatusHarness(budget: BudgetViewModel, item: BudgetItemViewModel) -> Element {
    use_context_provider(|| BudgetState(Signal::new(budget.clone())));
    rsx! {
        BudgetItemStatusView { item: item.clone() }
    }
}

fn item_with_status(status: BudgetItemStatus) -> BudgetItemViewModel {
    BudgetItemViewModel {
        name: "Livsmedel".to_string(),
        budgeting_type: BudgetingType::Expense,
        budgeted_amount: Money::new_dollars(1000, Currency::SEK),
        actual_amount: Money::new_dollars(1200, Currency::SEK),
        status,
        ..Default::default()
    }
}

#[test]
fn status_view_balanced_renders_nothing() {
    let html = render(rsx! {
        StatusHarness {
            budget: BudgetViewModel::default(),
            item: item_with_status(BudgetItemStatus::Balanced),
        }
    });
    // Balanced -> empty rsx: none of the status indicators appear.
    assert!(!html.contains("over-budget-indicator"));
    assert!(!html.contains("auto-adjust-button"));
}

#[test]
fn status_view_not_budgeted_shows_indicator() {
    let html = render(rsx! {
        StatusHarness {
            budget: BudgetViewModel::default(),
            item: item_with_status(BudgetItemStatus::NotBudgeted),
        }
    });
    assert!(html.contains("Ej budgeterad"));
}

#[test]
fn status_view_over_budget_shows_adjust_button() {
    let html = render(rsx! {
        StatusHarness {
            budget: BudgetViewModel::default(),
            item: item_with_status(BudgetItemStatus::OverBudget),
        }
    });
    assert!(html.contains("Över budget"));
    assert!(html.contains("auto-adjust-button"));
    // shortage = actual (1200) - budgeted (1000) = 200 kr; label reflects it.
    assert!(html.contains("Auto-justera"));
}
