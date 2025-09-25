use api::cqrs::budget_item::BudgetItem;
use dioxus::prelude::*;
use dioxus::logger::tracing;
#[component]
pub fn BudgetItemView(item: BudgetItem, index: usize) -> Element {
    tracing::info!("item_name: {}", item.name);
    tracing::info!("item_type: {}", item.item_type.to_string());
    tracing::info!("item_amount: {}", item.budgeted_amount.to_string());
    let gaah =format!("{}{}{}",item.name, item.item_type.to_string(), item.budgeted_amount.to_string());
    
    let item_amount = use_signal(|| item.budgeted_amount.to_string());
    let item_type = use_signal(|| item.item_type.to_string());
    
    rsx! {
        div {
            h1 { {item.name} }
            h2 { {item.item_type.to_string()} }
            h2 { {item.budgeted_amount.to_string()} }
            h2 { {item.actual_spent.to_string()} }
        }
    }
}
