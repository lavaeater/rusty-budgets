use api::cqrs::budget_item::BudgetItem;
use api::cqrs::budgeting_type::BudgetingType;
use dioxus::logger::tracing;
use dioxus::prelude::*;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType, index: usize) -> Element {
    tracing::info!("item_name: {}", item.name);
    tracing::info!("item_type: {}", item_type.to_string());
    tracing::info!("item_amount: {}", item.budgeted_amount.to_string());

    let item_amount = use_signal(|| item.budgeted_amount.to_string());
    let item_type = use_signal(|| item_type.to_string());

    rsx! {
        div {
            h1 { {item.name} }
            h2 { {item_type.to_string()} }
            h2 { {item.budgeted_amount.to_string()} }
            h2 { {item.actual_spent.to_string()} }
        }
    }
}
