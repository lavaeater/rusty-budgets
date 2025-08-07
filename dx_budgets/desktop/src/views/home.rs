use crate::Route;
use dioxus::prelude::*;
use lucide_dioxus::Plus;
use ui::{BudgetHero, Users};
use ui::budget::budget_hero::DEFAULT_BUDGET_ID;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
        Link { to: Route::NewBudgetItem {
            budget_id: *DEFAULT_BUDGET_ID.read(),
            item_type: "income".to_string(),
        },
            Plus {
                size: 48,
                color: "green",
            }
        }
        Users {}
    }
}
