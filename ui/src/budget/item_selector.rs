use dioxus::prelude::*;
use strum::{EnumCount, IntoEnumIterator};
use api::models::BudgetItem;
use api::view_models::BudgetItemViewModel;
use crate::*;

#[component]
pub fn ItemSelector(items: Vec<BudgetItemViewModel>, on_change: EventHandler<Option<BudgetItemViewModel>>) -> Element {
    items.sort_by_key(|it| it.name.clone());
    let selector_items = items
        .into_iter()
        .enumerate()
        .map(|(ix, it)| {
        rsx! {
            SelectOption::<BudgetItemViewModel> { index: ix, value: it.clone(), text_value: "{it.name}",
                {it.name.clone()}
                SelectItemIndicator {}
            }
        }
    });

    rsx! {
        Select::<BudgetItemViewModel> {
            placeholder: "Välj en budgetpost",
            on_value_change: move |e| on_change.call(e),
            SelectTrigger { aria_label: "Select Trigger", width: "12rem", SelectValue {} }
            SelectList { aria_label: "Select Demo",
                SelectGroup {
                    SelectGroupLabel { "Budgetposter" }
                    {selector_items}
                }
            }
        }
    }
}
