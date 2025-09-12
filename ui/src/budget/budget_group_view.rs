use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::accordion::{AccordionContent, AccordionItem, AccordionTrigger};
use api::cqrs::budget::BudgetGroup;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetGroupView(group: BudgetGroup, index: usize) -> Element {
    let mut show_new_item = use_signal(|| false);
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new(0, "SEK"));
    
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
                    if show_new_item() {
                        div { id: "new_item",
                            label { "Ny budgetpost" }
                            input {
                                r#type: "text",
                                placeholder: "Budgetpostnamn",
                                oninput: move |e| { new_item_name.set(e.value()) },
                            
                            }
                            input {
                                r#type: "number",
                                placeholder: "Budgetpostbelopp",
                                oninput: move |e| { new_item_amount.set(Money::new(e.value().parse().unwrap(), "SEK")) },
                            }
                        }
                    } else {
                        button {
                            class: "button",
                            "data-style": "primary",
                            onclick: move |_| {
                                show_new_item.set(true);
                            },
                            "Ny budgetpost"
                        }
                    }
                }
            }
        }
    }
}