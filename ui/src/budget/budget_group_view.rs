use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::accordion::{AccordionContent, AccordionItem, AccordionTrigger};
use api::cqrs::budget::BudgetGroup;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetGroupView(group: BudgetGroup, index: usize) -> Element {
    rsx! {
        AccordionItem {
            class: "accordion-item",
            index,
            on_change: move |open| {
                tracing::info!("{open};");
            },
            on_trigger_click: move || {
                tracing::info!("trigger");
            },
            AccordionTrigger { class: "accordion-trigger",
                {group.name.clone()}
                svg {
                    class: "accordion-expand-icon",
                    view_box: "0 0 24 24",
                    xmlns: "http://www.w3.org/2000/svg",
                    polyline { points: "6 9 12 15 18 9" }
                }
            }
            AccordionContent {
                class: "accordion-content",
                style: "--collapsible-content-width: 140px",
                div { padding_bottom: "1rem",
                    p { padding: "0", {group.name.clone()} }
                }
            }
        }
    }
}