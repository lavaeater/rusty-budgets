use dioxus::prelude::*;
use api::cqrs::budget::BudgetItem;

#[component]
pub fn BudgetItemView(item: BudgetItem, index: usize) -> Element {
    rsx! {
        h3 { {item.name} }
        h5 { {item.item_type.to_string()} }
        h5 { {item.budgeted_amount.to_string()} }
    }
}