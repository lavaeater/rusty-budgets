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
        let pct = u32::try_from((actual * 100 / budgeted).min(100)).unwrap_or(100);
        let over = actual > budgeted;
        (pct, over)
    } else {
        (0, false)
    };

    let bar_class = if bar_over { "overview-bar-fill over" } else { "overview-bar-fill" };
    let card_modifier = if overview.is_ok { "" } else { "over-budget" };
    let title_class = if overview.is_ok { "" } else { "warning" };

    rsx! {
        div { class: format!("overview-card {card_modifier}"),
            h3 { class: title_class, {budgeting_type.to_string()} }
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
