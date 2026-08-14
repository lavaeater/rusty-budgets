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
use ui::budget::{
    BudgetItemStatusView, BudgetState, BudgetTab, BudgetWorkspace, BudgetingTypeOverviewView,
};

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

// ---------------------------------------------------------------------------
// Workspace tabs (Phase 7). The tab bar is the app's primary navigation, so
// these lock down that every task view is reachable and that the Att göra
// badge reflects the outstanding-work counts.
// ---------------------------------------------------------------------------

#[component]
fn WorkspaceHarness(budget: BudgetViewModel) -> Element {
    use_context_provider(|| BudgetState(Signal::new(budget.clone())));
    rsx! {
        BudgetWorkspace {
            budget_id: Signal::new(budget.id),
            period_id: Signal::new(budget.period_id),
            tab: Signal::new(BudgetTab::default()),
        }
    }
}

fn workspace_html(budget: BudgetViewModel) -> String {
    render(rsx! {
        WorkspaceHarness { budget }
    })
}

#[test]
fn workspace_renders_every_tab() {
    let html = workspace_html(BudgetViewModel {
        name: "Hushåll".to_string(),
        ..Default::default()
    });

    for tab in [
        "Översikt",
        "Budget",
        "Transaktioner",
        "Att göra",
        "Rapporter",
        "Inställningar",
    ] {
        assert!(html.contains(tab), "tab bar is missing `{tab}`:\n{html}");
    }
    assert!(html.contains("Hushåll"), "budget name should be the heading");
}

#[test]
fn workspace_defaults_to_the_overview_tab() {
    let html = workspace_html(BudgetViewModel::default());
    // Only the selected panel renders its children, so the Översikt-only
    // "all clear" message proves Översikt is the active tab.
    assert!(
        html.contains("Allt är hanterat"),
        "overview panel should be the one rendered:\n{html}"
    );
}

#[test]
fn todo_badge_sums_outstanding_work() {
    let html = workspace_html(BudgetViewModel {
        untagged_transaction_count: 7,
        potential_transfer_count: 3,
        ..Default::default()
    });
    assert!(
        html.contains(r#"class="workspace-tab-badge">10<"#),
        "Att göra badge should total 7 + 3:\n{html}"
    );
}

#[test]
fn no_todo_badge_when_nothing_outstanding() {
    let html = workspace_html(BudgetViewModel::default());
    assert!(
        !html.contains("workspace-tab-badge"),
        "a clean budget should carry no badge:\n{html}"
    );
}

#[test]
fn overview_lists_work_needing_attention() {
    let html = workspace_html(BudgetViewModel {
        untagged_transaction_count: 4,
        ..Default::default()
    });
    assert!(html.contains("transaktioner att tagga"));
    assert!(
        !html.contains("Allt är hanterat"),
        "the all-clear message and the attention list are mutually exclusive"
    );
}

#[component]
fn TabHarness(budget: BudgetViewModel, tab: BudgetTab) -> Element {
    use_context_provider(|| BudgetState(Signal::new(budget.clone())));
    rsx! {
        BudgetWorkspace {
            budget_id: Signal::new(budget.id),
            period_id: Signal::new(budget.period_id),
            tab: Signal::new(tab),
        }
    }
}

/// A budget with no projected overviews must not panic when the Budget tab is
/// opened — `BudgetingTypeTabs` used to `.unwrap()` the first overview.
#[test]
fn budget_tab_survives_an_empty_view_model() {
    let html = render(rsx! {
        TabHarness { budget: BudgetViewModel::default(), tab: BudgetTab::Plan }
    });
    assert!(html.contains("Ingen budgetdata"), "expected the empty state:\n{html}");
}

#[test]
fn every_tab_renders_without_panicking() {
    for tab in [
        BudgetTab::Overview,
        BudgetTab::Plan,
        BudgetTab::Transactions,
        BudgetTab::Todo,
        BudgetTab::Reports,
        BudgetTab::Settings,
    ] {
        let html = render(rsx! {
            TabHarness { budget: BudgetViewModel::default(), tab }
        });
        assert!(!html.is_empty(), "tab {tab:?} rendered nothing");
    }
}
