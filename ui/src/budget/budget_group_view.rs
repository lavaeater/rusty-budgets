use crate::budget_item_view::BudgetItemView;
use crate::components::*;
use api::cqrs::budget::{BudgetGroup, BudgetItemType};
use api::cqrs::money::{Currency, Money};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::select::*;
use uuid::Uuid;
#[component]
pub fn BudgetGroupView(budget_id: Uuid, group: BudgetGroup, index: usize) -> Element {
    let mut budget_items = use_signal(|| group.items.clone());
    let mut show_new_item = use_signal(|| budget_items().is_empty());
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));
    let new_item_type = use_signal(|| Some(None));

    rsx! {
        AccordionItem {
            index,
            height: "100",
            on_change: move |open| {
                tracing::info!("{open};");
            },
            on_trigger_click: move || {
                tracing::info!("trigger");
            },
            AccordionTrigger { {group.name.clone()} }
            AccordionContent {
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
                                oninput: move |e| {
                                    new_item_amount
                                        .set(Money::new_dollars(e.value().parse().unwrap(), Currency::SEK))
                                },
                            }
                            ItemTypeSelect { selected_value: new_item_type }
                            button {
                                class: "button",
                                "data-style": "primary",
                                onclick: move |_| async move {
                                    let item_type = new_item_type().unwrap().unwrap();
                                    if let Ok(items) = api::add_item(
                                            budget_id,
                                            group.id,
                                            new_item_name(),
                                            item_type,
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
                            "Ny budgetpost"
                        }
                    }
                }
                for (index , item) in budget_items().iter().enumerate() {
                    BudgetItemView { item: item.clone(), index }
                }
            }
        }
    }
}

#[component]
pub fn ItemTypeSelect(mut selected_value: Signal<Option<Option<BudgetItemType>>>) -> Element {
    rsx! {
        Select::<BudgetItemType> {
            placeholder: "Välj typ",
            on_value_change: move |value: Option<BudgetItemType>| {
                selected_value.set(Some(value));
                if let Some(val) = value {
                    tracing::info!("Selected value: {:?}", val);
                }
            },
            SelectTrigger { aria_label: "Väljare", width: "12rem", SelectValue {} }
            SelectList { aria_label: "Typväljare",
                SelectOption::<BudgetItemType> {
                    index: 0usize,
                    value: BudgetItemType::Income,
                    text_value: "Inkomst",
                    "Inkomst"
                    SelectItemIndicator { "✔️" }
                }
                SelectOption::<BudgetItemType> {
                    index: 1usize,
                    value: BudgetItemType::Expense,
                    text_value: "Utgift",
                    "Utgift"
                    SelectItemIndicator { "✔️" }
                }
                SelectOption::<BudgetItemType> {
                    index: 2usize,
                    value: BudgetItemType::Savings,
                    text_value: "Sparande",
                    "Sparande"
                    SelectItemIndicator { "✔️" }
                }
            }
        }
    }
}
