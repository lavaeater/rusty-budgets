use crate::Route;
use api::models::budget::Budget;
use dioxus::prelude::*;
use lucide_dioxus::Plus;
use uuid::Uuid;
use ui::budget::budget_hero::DEFAULT_BUDGET_ID;
use ui::{BudgetHero, Users};
use api::*;

#[component]
pub fn Home() -> Element {
    let mut budget = use_server_future(api::get_default_budget)?;

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
