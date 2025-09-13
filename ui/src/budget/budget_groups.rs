use api::cqrs::budget::BudgetGroup;
use dioxus::prelude::*;
use crate::components::Accordion;
use crate::budget::budget_group_view::BudgetGroupView;
use crate::budget_hero::CURRENT_BUDGET_ID;

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
                onclick: move |_| {
                    show_new_group.set(true);
                },
                "Lägg till ny grupp"
            }
        }
        Accordion {
            width: "40%",
            allow_multiple_open: false,
            horizontal: false,
            for (index , group) in budget_groups().iter().enumerate() {
                BudgetGroupView { group: group.clone(), index }
            }
        }
    }
}