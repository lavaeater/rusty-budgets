use api::cqrs::budget::BudgetItem;
use dioxus::prelude::*;

#[component]
pub fn BudgetItemView(item: BudgetItem, index: usize) -> Element {
    rsx! {
        h3 { {item.name} }
        h3 { {item.item_type.to_string()} }
        h3 { {item.budgeted_amount.to_string()} }
    }
}
