use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::select::*;
use crate::budget_components::{Collapsible, CollapsibleContent, CollapsibleTrigger};

use uuid::Uuid;
use api::models::{BudgetItem, BudgetingType, Currency, Money};
use crate::{BudgetItemView, Separator};

#[component]
pub fn BudgetingTypeCard(budget_id: Uuid, budgeting_type: BudgetingType, items: Vec<BudgetItem>) -> Element {
    let budgeting_type_name = use_signal(|| budgeting_type.to_string());
    let mut budget_items = use_signal(|| items);
    let mut show_new_item = use_signal(|| budget_items().is_empty());
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));

    rsx! {
        h3 { {budgeting_type_name} }
        div { padding_bottom: "1rem",
            p { padding: "0", {budgeting_type_name} }
            if show_new_item() {
                div { id: "new_item",
                    label { "Ny post" }
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
                            if let Ok(items) = api::add_item(
                                    budget_id,
                                    new_item_name(),
                                    budgeting_type,
                                    new_item_amount(),
                                )
                                .await
                            {
                                budget_items.set(items);
                            }
                            show_new_item.set(false);
                        },
                        "Lägg till"
                    }
                }
            } else {
                button {
                    class: "button",
                    "data-style": "primary",
                    onclick: move |_| {
                        show_new_item.set(true);
                    },
                    "Ny post"
                }
            }
        }
        for (index , item) in budget_items().iter().enumerate() {
            BudgetItemView { item: item.clone(), item_type: budgeting_type, index }
            Separator {
                style: "margin: 15px 0; width: 50%;",
                horizontal: true,
                decorative: true,
            }
        }
    }
        }


// 
// #[component]
// pub fn ItemTypeSelect(mut selected_value: Signal<Option<Option<BudgetingType>>>) -> Element {
//     rsx! {
//         Select::<BudgetingType> {
//             placeholder: "Välj typ",
//             on_value_change: move |value: Option<BudgetingType>| {
//                 selected_value.set(Some(value));
//                 if let Some(val) = value {
//                     tracing::info!("Selected value: {:?}", val);
//                 }
//             },
//             SelectTrigger { aria_label: "Väljare", width: "12rem", SelectValue {} }
//             SelectList { aria_label: "Typväljare",
//                 SelectOption::<BudgetingType> {
//                     index: 0usize,
//                     value: BudgetingType::Income,
//                     text_value: "Inkomst",
//                     "Inkomst"
//                     SelectItemIndicator { "✔️" }
//                 }
//                 SelectOption::<BudgetingType> {
//                     index: 1usize,
//                     value: BudgetingType::Expense,
//                     text_value: "Utgift",
//                     "Utgift"
//                     SelectItemIndicator { "✔️" }
//                 }
//                 SelectOption::<BudgetingType> {
//                     index: 2usize,
//                     value: BudgetingType::Savings,
//                     text_value: "Sparande",
//                     "Sparande"
//                     SelectItemIndicator { "✔️" }
//                 }
//             }
//         }
//     }
// }
