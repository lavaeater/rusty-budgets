use dioxus::logger::tracing;
use crate::budget_popover::BudgetPopover;
use api::cqrs::budget::{Budget, BudgetGroup};
use api::models::*;
use dioxus::prelude::*;
use dioxus_primitives::accordion::{Accordion, AccordionContent, AccordionItem, AccordionTrigger};
use uuid::Uuid;
use crate::budget_hero::CURRENT_BUDGET_ID;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetGroups(groups: Vec<BudgetGroup>) -> Element {
    let budget_id = *CURRENT_BUDGET_ID.read();
    let mut group_name = use_signal(|| "".to_string());
    let mut budget_groups = use_signal(|| groups);
    let mut show_new_group = use_signal(|| false);
    rsx! {
        if show_new_group() {
            div { id: "new_group",
                label { "Skapa ny grupp" }
                input {
                    r#type: "text",
                    placeholder: "Gruppnamn",
                    oninput: move |e| { group_name.set(e.value()) },
                }
                button {
                    class: "button",
                    "data-style": "primary",
                    onclick: move |_| async move {
                        if let Ok(budget) = api::add_group(budget_id, group_name.to_string()).await {
                            budget_groups.set(budget);
                        }
                        show_new_group.set(false);
                    },
                    "Skapa grupp"
                }
            }
        } else {
            button {
                class: "button",
                    "data-style": "primary",
                    onclick: move |_|  {
                        show_new_group.set(true);
                    },
                    "Lägg till ny grupp"
            }
        }
        Accordion {
            class: "accordion",
            width: "15rem",
            allow_multiple_open: false,
            horizontal: false,
            for (i , group) in budget_groups().iter().enumerate() {
                AccordionItem {
                    class: "accordion-item",
                    index: i,
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
    }
}