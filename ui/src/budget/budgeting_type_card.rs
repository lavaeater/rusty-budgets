use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::select::*;
use crate::components::{Collapsible, CollapsibleContent, CollapsibleTrigger};
use uuid::Uuid;
use api::models::BudgetingType;
use api::view_models::*;
use crate::{Button, Separator};
use crate::budget::{BudgetItemView, NewBudgetItem};

#[component]
pub fn BudgetingTypeCard(budgeting_type: BudgetingType, items: Vec<BudgetItemViewModel>) -> Element {
    info!("Budgeting type: {}, item count: {}", budgeting_type, items.len());
    let budgeting_type_name = use_signal(|| budgeting_type.to_string());
    let new_item_label = format!("Ny {}", budgeting_type);
    let mut show_new_item = use_signal(|| items.is_empty());
    
    rsx! {
        h3 { {budgeting_type_name} }
        div { padding_bottom: "1rem",
            if show_new_item() {
                NewBudgetItem { budgeting_type, close_signal: Some(show_new_item) }
            } else {
                Button {
                    class: "button",
                    "data-style": "primary",
                    onclick: move |_| {
                        show_new_item.set(true);
                    },
                    {{ new_item_label }}
                }
            }
        }
        for item in items {
            BudgetItemView {
                item: item.clone(),
                item_type: budgeting_type,
            }
            Separator {
                style: "margin: 15px 0; width: 50%;",
                horizontal: true,
                decorative: true,
            }
        }
    }
}
