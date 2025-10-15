use dioxus::prelude::*;
use ui::budget_a::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
