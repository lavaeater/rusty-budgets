use dioxus::prelude::*;
use ui::BudgetHero;

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
    }
}
