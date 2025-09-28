use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::select::*;
use crate::budget_components::{Collapsible, CollapsibleContent, CollapsibleTrigger};

use uuid::Uuid;
use api::models::{BudgetItem, BudgetingType, Currency, Money};
use crate::{BudgetItemView, Button, Separator};
use crate::budget::budget_hero::CURRENT_BUDGET_UPDATED;

#[component]
pub fn BudgetingTypeCard(budget_id: Uuid, budgeting_type: BudgetingType, items: Vec<BudgetItem>) -> Element {
    let budgeting_type_name = use_signal(|| budgeting_type.to_string());
    let new_item_label = format!("Ny {}", budgeting_type);
    let mut budget_items = use_signal(|| items);
    let mut show_new_item = use_signal(|| budget_items().is_empty());
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));

    rsx! {
        h3 { {budgeting_type_name} }
        div { padding_bottom: "1rem",
            if show_new_item() {
                div { id: "new_item",
                    label { {new_item_label} }
                    input {
                        r#type: "text",
                        placeholder: "Namn",
                        oninput: move |e| { new_item_name.set(e.value()) },
                    }
                    input {
                        r#type: "number",
                        placeholder: "Belopp",
                        oninput: move |e| {
                            new_item_amount
                                .set(Money::new_dollars(e.value().parse().unwrap(), Currency::SEK))
                        },
                    }
                    button {
                        class: "button",
                        "data-style": "primary",
                        onclick: move |_| async move {
                            if let Ok(_) = api::add_item(
                                    budget_id,
                                    new_item_name(),
                                    budgeting_type,
                                    new_item_amount(),
                                )
                                .await
                            {
                                *CURRENT_BUDGET_UPDATED.write() = true;
                            }
                            show_new_item.set(false);
                        },
                        "Lägg till"
                    }
                }
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
        for item in budget_items() {
            BudgetItemView { item: item.clone(), item_type: budgeting_type }
            Separator {
                style: "margin: 15px 0; width: 50%;",
                horizontal: true,
                decorative: true,
            }
        }
    }
        }
