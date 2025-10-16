use dioxus::prelude::*;
use ui::budget::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
