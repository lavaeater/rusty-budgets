use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::accordion::{AccordionContent, AccordionItem, AccordionTrigger};
use dioxus_primitives::select::*;
use api::cqrs::budget::{BudgetGroup, BudgetItemType};
use api::cqrs::money::{Currency, Money};

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetGroupView(group: BudgetGroup, index: usize) -> Element {
    let mut show_new_item = use_signal(|| false);
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new(0, Currency::SEK));
    let mut new_item_type = use_signal(|| Some(None));
    
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
                                oninput: move |e| {
                                    new_item_amount.set(Money::new(e.value().parse().unwrap(), Currency::SEK))
                                },
                            }
                            ItemTypeSelect { selected_value: new_item_type }
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

#[component]
pub fn ItemTypeSelect(mut selected_value: Signal<Option<Option<BudgetItemType>>>) -> Element {

    rsx! {
        Select::<String> {
            placeholder: "Select a fruit...",
            SelectTrigger {
                aria_label: "Select Trigger",
                width: "12rem",
                SelectValue {}
            }
            SelectList {
                aria_label: "Select Demo",
                SelectGroup {
                    SelectGroupLabel { "Fruits" }
                    SelectOption::<String> {
                        index: 0usize,
                        value: "apple",
                        "Apple"
                        SelectItemIndicator { "✔️" }
                    }
                    SelectOption::<String> {
                        index: 1usize,
                        value: "banana",
                        "Banana"
                        SelectItemIndicator { "✔️" }
                    }
                }
            }
        }
        // Select::<BudgetItemType> {
        //     placeholder: "Välj typ",
        //     // The currently selected value(s) in the dropdown.
        //     // Callback function triggered when the selected value changes.
        //     on_value_change: move |value: Option<BudgetItemType>| {
        //         selected_value.set(Some(value));
        //         if let Some(val) = value {
        //             tracing::info!("Selected value: {:?}", val);
        //         }
        //     },
        //     // The select trigger is the button that opens the dropdown.
        //     SelectTrigger {
        //         // The (optional) select value displays the currently selected text value.
        //         SelectValue {}
        //     }
        //     // All groups must be wrapped in the select list.
        //     SelectList {
        //         // Each select option represents an individual option in the dropdown. The type must match the type of the select.
        //         SelectOption::<BudgetItemType> {
        //             index: 0usize,
        //             // The value of the item, which will be passed to the on_value_change callback when selected.
        //             value: BudgetItemType::Income,
        //             text_value: "Inkomst",
        //         }
        //         SelectOption::<BudgetItemType> {
        //             index: 1usize,
        //             // The value of the item, which will be passed to the on_value_change callback when selected.
        //             value: BudgetItemType::Expense,
        //             text_value: "Utgift",
        //         }
        //         SelectOption::<BudgetItemType> {
        //             index: 2usize,
        //             // The value of the item, which will be passed to the on_value_change callback when selected.
        //             value: BudgetItemType::Savings,
        //             text_value: "Sparande",
        //         }
        //     }
        // }
    }
}