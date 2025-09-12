use dioxus::prelude::*;
use api::cqrs::budget::BudgetItem;

#[component]
pub fn BudgetItemView(item: BudgetItem, index: usize) -> Element {
    rsx! {
        h4 { {item.name} }
    }
}