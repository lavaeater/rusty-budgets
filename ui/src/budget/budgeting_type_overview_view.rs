use api::models::{Budget, BudgetingType};
use api::view_models::BudgetingTypeOverview;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetingTypeOverviewView(
    budgeting_type: BudgetingType,
    overview: BudgetingTypeOverview,
) -> Element {
    // Progress = actual / budgeted, capped at 100% for the bar width.
    // For Income the bar shows how much of income is allocated (expense+savings/budgeted).
    let (bar_pct, bar_over) = if overview.budgeted_amount.amount_in_cents() > 0 {
        let actual = overview.actual_amount.amount_in_cents().abs();
        let budgeted = overview.budgeted_amount.amount_in_cents().abs();
        let pct = (actual * 100 / budgeted).min(100) as u32;
        let over = actual > budgeted;
        (pct, over)
    } else {
        (0, false)
    };

    let bar_class = if bar_over { "overview-bar-fill over" } else { "overview-bar-fill" };

    rsx! {
        div { class: format!("overview-card {}", if !overview.is_ok { "over-budget" } else { "" }),
            h3 { class: if !overview.is_ok { "warning" } else { "" }, {budgeting_type.to_string()} }
            div { class: "overview-progress-bar",
                div { class: bar_class, style: "width: {bar_pct}%;" }
            }
            div { class: "card-stats",
                div { class: "stat",
                    span { class: "stat-label", "Budgeterat" }
                    span { class: "stat-value", {overview.budgeted_amount.to_string()} }
                }
                div { class: "stat",
                    span { class: "stat-label", "Faktiskt" }
                    span { class: "stat-value", {overview.actual_amount.to_string()} }
                }
                div { class: "stat",
                    span { class: "stat-label", "Återstår" }
                    span { class: "stat-value stat-remaining", {overview.remaining_budget.to_string()} }
                }
            }
        }
    }
}
