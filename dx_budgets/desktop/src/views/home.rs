use dioxus::prelude::*;
use ui::{BudgetHero, Users};
use lucide_dioxus::Plus;
use crate::Route;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
        Link { to: Route::NewBudgetItem {}, 
            Plus {
                size: 48,
                color: "green",
            }
        }
        Users {}
    }
}
