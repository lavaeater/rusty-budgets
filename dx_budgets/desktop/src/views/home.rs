use dioxus::prelude::*;
use ui::{BudgetHero, Users};
use lucide_dioxus::Plus;
use ui::budget::budget_hero::DEFAULT_BUDGET_ID;
use crate::Route;

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
