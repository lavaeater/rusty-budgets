use api::cqrs::budget::BudgetItem;
use dioxus::prelude::*;
use dioxus::logger::tracing;
#[component]
pub fn BudgetItemView(item: BudgetItem, index: usize) -> Element {
    tracing::info!("item_name: {}", item.name);
    tracing::info!("item_type: {}", item.item_type);
    tracing::info!("item_amount: {}", item.budgeted_amount.to_string());
    
    let item_amount = use_signal(|| item.budgeted_amount.to_string());
    let item_type = use_signal(|| item.item_type.to_string());
    
    rsx! {
        div {
            h3 { {item.name} }
            p { {item_type} }
            p { {item_amount} }
        }
    }
}
