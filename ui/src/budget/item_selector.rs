use dioxus::prelude::*;
use strum::{EnumCount, IntoEnumIterator};
use api::models::BudgetItem;
use crate::*;

#[component]
pub fn ItemSelect(items: Vec<BudgetItem>) -> Element {
    let items = items
        .into_iter()
        .enumerate()
        .map(|(ix, it)| {
        rsx! {
            SelectOption::<Option<BudgetItem>> { index: ix, value: it.clone(), text_value: "{it.name}",
                {it.name.clone()}
                SelectItemIndicator {}
            }
        }
    });

    rsx! {

        Select::<Option<BudgetItem>> { placeholder: "Välj en budgetpost",
            SelectTrigger { aria_label: "Select Trigger", width: "12rem", SelectValue {} }
            SelectList { aria_label: "Select Demo",
                SelectGroup {
                    SelectGroupLabel { "Budgetposter" }
                    {items}
                }
            }
        }
    }
}
