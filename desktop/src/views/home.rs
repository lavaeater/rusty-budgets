use crate::Route;
use dioxus::prelude::*;
use lucide_dioxus::Plus;
use ui::BudgetHero;
use ui::budget::budget_hero::CURRENT_BUDGET_ID;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
