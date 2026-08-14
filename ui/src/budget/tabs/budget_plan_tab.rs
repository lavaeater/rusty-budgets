use crate::budget::budget_hero::BudgetState;
use crate::budget::{BudgetingTypeTabs, CreateBudgetItemsView};
use api::models::BudgetingType;
use dioxus::prelude::*;

/// The envelope table — assign money, adjust amounts, move funds between items.
///
/// This is the day-to-day surface once the tagging backlog is clear.
#[component]
pub fn BudgetPlanTab() -> Element {
    let budget = use_context::<BudgetState>().0();

    let ready_to_assign = budget
        .overviews
        .iter()
        .find(|ov| ov.budgeting_type == BudgetingType::Income)
        .map(|ov| ov.remaining_budget);

    let has_income = budget
        .items
        .iter()
        .any(|i| i.budgeting_type == BudgetingType::Income);

    // Live tags that no budget item covers yet — the signal that the
    // item-creation workflow still has work to do. `CreateBudgetItemsView` does
    // the authoritative server-side query; this is just enough to decide whether
    // the section opens by default.
    let unbudgeted_tag_count = budget
        .tags
        .iter()
        .filter(|tag| !tag.deleted)
        .filter(|tag| {
            !budget
                .items
                .iter()
                .any(|item| item.tag_ids.contains(&tag.id))
        })
        .count();

    rsx! {
        div { class: "tab-panel",
            if let Some(rta) = ready_to_assign
                && rta.amount_in_cents() > 0
                && has_income
            {
                div { class: "assign-nudge",
                    span { class: "assign-nudge-icon", "💡" }
                    span {
                        "Du har "
                        strong { {rta.to_string()} }
                        " kvar att fördela. Tilldela dem till en budgetpost så att varje krona har ett syfte."
                    }
                }
            }

            div { class: "budget-main-content", BudgetingTypeTabs {} }

            details {
                class: "workflow-section",
                open: unbudgeted_tag_count > 0,
                summary { class: "workflow-section-trigger",
                    "Skapa budgetposter"
                    if unbudgeted_tag_count > 0 {
                        span { class: "workflow-section-badge", {unbudgeted_tag_count.to_string()} }
                    }
                }
                div { class: "workflow-section-content", CreateBudgetItemsView {} }
            }
        }
    }
}
