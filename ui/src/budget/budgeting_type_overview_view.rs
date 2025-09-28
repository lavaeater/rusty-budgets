use api::models::{BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetingTypeOverviewView(
    budget_id: Uuid,
    budgeting_type: BudgetingType,
    overview: BudgetingTypeOverview,
) -> Element {
    rsx! {
        div {
            h2 { {budgeting_type.to_string()} }
            p { {overview.budgeted_amount.to_string()} }
            p { {overview.actual_amount.to_string()} }
            p { {overview.remaining_budget.to_string()} }
        }
    }
}
