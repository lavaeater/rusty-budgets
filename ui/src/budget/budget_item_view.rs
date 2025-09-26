use dioxus::fullstack::server_fn::serde::{Deserialize, Serialize};
use api::models::*;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetItemView(item: BudgetItem, item_type: BudgetingType) -> Element {
    rsx! {
        div {
            class: "flex justify-between items-center p-2 border-b border-gray-200 text-sm",

            // Left side: name
            div { class: "font-medium",
                "{item.name}"
            }

            // Right side: actual / budgeted
            div { class: "text-gray-700",
                "{item.spent_amount.to_string()} / {item.budgeted_amount.to_string()}"
            }
        }
    }
}