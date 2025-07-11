use dioxus::prelude::*;
use ui::{Users, Hero, BudgetHero};

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHero {}
        Users {}
    }
}
