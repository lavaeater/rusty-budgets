use dioxus::prelude::*;
use strum::{EnumCount, IntoEnumIterator};
use api::models::BudgetItem;
use crate::*;

#[component]
pub fn ItemSelector(items: Vec<BudgetItem>, on_change: EventHandler<Option<BudgetItem>>) -> Element {
    
    let items = items
        .into_iter()
        .enumerate()
        .map(|(ix, it)| {
        rsx! {
            SelectOption::<BudgetItem> { index: ix, value: it.clone(), text_value: "{it.name}",
                {it.name.clone()}
                SelectItemIndicator {}
            }
        }
    });

    rsx! {
        Select::<BudgetItem> {
            placeholder: "Välj en budgetpost",
            on_value_change: move |e| on_change.call(e),
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
