use uuid::Uuid;
use api::cqrs::budget::BudgetGroup;
use dioxus::prelude::*;
use dioxus_primitives::collapsible::Collapsible;
use crate::budget_components::Accordion;
use crate::budget::budget_group_view::BudgetGroupView;

#[component]
pub fn BudgetGroups(budget_id: Uuid, groups: Vec<BudgetGroup>) -> Element {
    let mut group_name = use_signal(|| "".to_string());
    let mut budget_groups = use_signal(|| groups);
    let mut show_new_group = use_signal(|| false);
    rsx! {
        if show_new_group() {
            div { id: "new_group", height: "100%",
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
                        if let Ok(returned_budgets) = api::add_group(budget_id, group_name.to_string())
                            .await
                        {
                            budget_groups.set(returned_budgets);
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
        div { display: "flex", flex_direction: "row", gap: "1rem",
            for (index , group) in budget_groups().iter().enumerate() {
                BudgetGroupView { budget_id, group: group.clone(), index }
            }
        }
    }
}