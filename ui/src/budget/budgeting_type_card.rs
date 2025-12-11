use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::select::*;
use crate::components::{Collapsible, CollapsibleContent, CollapsibleTrigger};
use uuid::Uuid;
use api::models::BudgetingType;
use api::view_models::*;
use crate::{Button, Separator};
use crate::budget::{BudgetItemView, NewBudgetItem};
use std::cmp::Ordering;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SortField {
    #[default]
    Name,
    BudgetedAmount,
    ActualAmount,
    RemainingBudget,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

#[component]
pub fn BudgetingTypeCard(budgeting_type: BudgetingType, items: Vec<BudgetItemViewModel>) -> Element {
    info!("Budgeting type: {}, item count: {}", budgeting_type, items.len());
    let budgeting_type_name = use_signal(|| budgeting_type.to_string());
    let new_item_label = format!("Ny {}", budgeting_type);
    let mut show_new_item = use_signal(|| items.is_empty());
    let mut sort_field = use_signal(SortField::default);
    let mut sort_direction = use_signal(SortDirection::default);

    let sorted_items = use_memo(move || {
        let mut sorted = items.clone();
        sorted.sort_by(|a, b| {
            let ordering = match sort_field() {
                SortField::Name => a.name.cmp(&b.name),
                SortField::BudgetedAmount => a.budgeted_amount.partial_cmp(&b.budgeted_amount).unwrap_or(Ordering::Equal),
                SortField::ActualAmount => a.actual_amount.partial_cmp(&b.actual_amount).unwrap_or(Ordering::Equal),
                SortField::RemainingBudget => a.remaining_budget.partial_cmp(&b.remaining_budget).unwrap_or(Ordering::Equal),
            };
            match sort_direction() {
                SortDirection::Ascending => ordering,
                SortDirection::Descending => ordering.reverse(),
            }
        });
        sorted
    });

    let mut handle_sort_click = move |field: SortField| {
        if sort_field() == field {
            sort_direction.set(match sort_direction() {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            });
        } else {
            sort_field.set(field);
            sort_direction.set(SortDirection::Ascending);
        }
    };

    let sort_indicator = |field: SortField| -> &'static str {
        if sort_field() == field {
            match sort_direction() {
                SortDirection::Ascending => " ↑",
                SortDirection::Descending => " ↓",
            }
        } else {
            ""
        }
    };

    rsx! {
        h3 { {budgeting_type_name} }
        div { padding_bottom: "1rem",
            if show_new_item() {
                NewBudgetItem { budgeting_type, close_signal: Some(show_new_item) }
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
        div {
            class: "sort-toolbar",
            style: "display: flex; gap: 0.5rem; margin-bottom: 1rem; flex-wrap: wrap;",
            Button {
                class: "button",
                "data-style": if sort_field() == SortField::Name { "primary" } else { "secondary" },
                onclick: move |_| handle_sort_click(SortField::Name),
                "Namn{sort_indicator(SortField::Name)}"
            }
            Button {
                class: "button",
                "data-style": if sort_field() == SortField::BudgetedAmount { "primary" } else { "secondary" },
                onclick: move |_| handle_sort_click(SortField::BudgetedAmount),
                "Budgeterat{sort_indicator(SortField::BudgetedAmount)}"
            }
            Button {
                class: "button",
                "data-style": if sort_field() == SortField::ActualAmount { "primary" } else { "secondary" },
                onclick: move |_| handle_sort_click(SortField::ActualAmount),
                "Faktiskt{sort_indicator(SortField::ActualAmount)}"
            }
            Button {
                class: "button",
                "data-style": if sort_field() == SortField::RemainingBudget { "primary" } else { "secondary" },
                onclick: move |_| handle_sort_click(SortField::RemainingBudget),
                "Återstår{sort_indicator(SortField::RemainingBudget)}"
            }
        }
        for item in sorted_items() {
            BudgetItemView { item: item.clone() }
            Separator {
                style: "margin: 15px 0; width: 50%;",
                horizontal: true,
                decorative: true,
            }
        }
    }
}
