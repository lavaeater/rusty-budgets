use dioxus::prelude::*;
use ui::{BudgetHeroOne};

#[component]
pub fn Home() -> Element {
    rsx! {
        BudgetHeroOne {}
    }
}
