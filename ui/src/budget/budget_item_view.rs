use crate::budget::ItemSelector;
use crate::{Button, ButtonVariant, Slider, SliderRange, SliderThumb, SliderTrack};
use api::ignore_transaction;
use api::models::{BudgetingType, Money};
use api::view_models::*;
use dioxus::prelude::*;
use lucide_dioxus::Pen;
use std::collections::HashSet;
use dioxus::logger::tracing;
use dioxus_primitives::slider::SliderValue;
use uuid::Uuid;
use api::view_models::BudgetItemStatus;
use api::view_models::BudgetItemViewModel;
use api::view_models::BudgetViewModel;
use crate::budget::budget_hero::BudgetState;
use crate::budget::BudgetItemStatusView;

#[component]
pub fn BudgetItemView(item: BudgetItemViewModel) -> Element {
    let mut expanded = use_signal(|| false);

    let mut edit_item = use_signal(|| false);
    let item_name = use_signal(|| item.name.clone());
    let mut budgeted_amount = use_signal(|| item.budgeted_amount);

    // State for selected transaction IDs and the target item for moving
    let mut selected_transactions = use_signal(HashSet::<Uuid>::new);
    let mut show_move_selector = use_signal(|| false);
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let remaining_to_budget = budget_signal().overviews.iter().find(|ov| ov.budgeting_type == BudgetingType::Income).unwrap().remaining_budget;
        if expanded() {
            rsx! {
                div { class: "budget-item-expanded", key: "{item.item_id}",
                    div {
                        class: "budget-item-expanded-header",
                        onclick: move |_| { expanded.set(false) },
                        div { class: "budget-item-expanded-name", "{item_name()}" }
                        div { class: "budget-item-expanded-amounts",
                            "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                        }
                    }
                    if item.transactions.is_empty() {
                        div { class: "no-transactions", "Inga transaktioner" }
                    } else {
                        table { class: "transaction-table",
                            thead {
                                tr {
                                    th { class: "checkbox-cell", "" }
                                    th { "Beskrivning" }
                                    th { "Datum" }
                                    th { "Belopp" }
                                }
                            }
                            tbody {
                                {
                                    item.transactions
                                        .iter()
                                        .map(|transaction| {
                                            let tx_id = transaction.tx_id;
                                            let is_selected = selected_transactions().contains(&tx_id);
                                            rsx! {
                                                tr { key: "{tx_id}",
                                                    td { class: "checkbox-cell",
                                                        input {
                                                            r#type: "checkbox",
                                                            checked: is_selected,
                                                            onchange: move |_| {
                                                                let mut selected = selected_transactions();
                                                                if is_selected {
                                                                    selected.remove(&tx_id);
                                                                } else {
                                                                    selected.insert(tx_id);
                                                                }
                                                                selected_transactions.set(selected);
                                                            },
                                                        }
                                                    }
                                                    td { {transaction.description.clone()} }
                                                    td { {transaction.date.format("%Y-%m-%d").to_string()} }
                                                    td { {transaction.amount.to_string()} }
                                                }
                                            }
                                        })
                                }
                            }
                        }
                    }
                    if !selected_transactions().is_empty() {
                        div { class: "transaction-actions-bar",
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| {
                                    selected_transactions.set(HashSet::new());
                                },
                                "Avmarkera alla"
                            }
                            Button {
                                variant: ButtonVariant::Destructive,
                                onclick: move |_| async move {
                                    let mut updated_budget: Option<BudgetViewModel> = None;
                                    let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                    for tx_id in selected_ids {
                                        if let Ok(ub) = ignore_transaction(
                                                budget_id,
                                                tx_id,
                                                budget_signal().period_id,
                                            )
                                            .await
                                        {
                                            updated_budget = Some(ub);
                                        } else {
                                            updated_budget = None;
                                            break;
                                        }
                                    }
                                    if let Some(updated_budget) = updated_budget {
                                        info!("Transactions ignored, budget updated");
                                        selected_transactions.set(HashSet::new());
                                        show_move_selector.set(false);
                                        consume_context::<BudgetState>().0.set(updated_budget);
                                    } else {
                                        error!("Transactions ignored, budget not updated");
                                        selected_transactions.set(HashSet::new());
                                        show_move_selector.set(false);
                                    }
                                },
                                "Ignorera alla"
                            }

                            if !show_move_selector() {
                                Button {
                                    variant: ButtonVariant::Primary,
                                    onclick: move |_| {
                                        show_move_selector.set(true);
                                    },
                                    "Flytta markerade"
                                }
                            } else {
                                div { class: "move-selector-container",
                                    span { class: "move-selector-label", "Flytta till:" }
                                    ItemSelector {
                                        items: budget_signal()
                                            .items
                                            .iter()
                                            .filter(|i| i.item_id != item.item_id)
                                            .cloned()
                                            .collect(),
                                        on_change: move |target_item: Option<BudgetItemViewModel>| async move {
                                            if let Some(target_item) = target_item {
                                                let mut success = true;
                                                let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                                for tx_id in selected_ids {
                                                    if api::connect_transaction(
                                                            budget_id,
                                                            tx_id,
                                                            target_item.actual_id,
                                                            target_item.item_id,
                                                            budget_signal().period_id,
                                                        )
                                                        .await
                                                        .is_err()
                                                    {
                                                        success = false;
                                                        break;
                                                    }
                                                }
                                                if success {
                                                    if let Ok(Some(updated_budget)) = api::get_budget(
                                                            Some(budget_id),
                                                            budget_signal().period_id,
                                                        )
                                                        .await
                                                    {
                                                        selected_transactions.set(HashSet::new());
                                                        show_move_selector.set(false);
                                                        consume_context::<BudgetState>().0.set(updated_budget);
                                                    } else {
                                                        show_move_selector.set(false);
                                                        error!("Transactions moved, budget not updated");
                                                    }
                                                }
                                            } else {
                                                show_move_selector.set(false);
                                            }
                                        },
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else if edit_item() {
            rsx! {
                div { class: "budget-item-edit", key: "{item.item_id}",
                    div { class: "budget-item-edit-header",
                        div { class: "budget-item-edit-title", "{item_name()}" }
                        div { class: "budget-item-edit-amounts",
                            "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                        }
                    }

                    div { class: "budget-item-edit-form",
                        div { class: "budget-item-edit-field",
                            label { class: "budget-item-edit-label", "Budgeterat belopp" }
                            input {
                                class: "budget-item-edit-input",
                                r#type: "number",
                                value: budgeted_amount().amount_in_dollars(),
                                oninput: move |e| {
                                    match e.value().parse() {
                                        Ok(v) => {
                                            budgeted_amount.set(Money::new_dollars(v, budget_signal().currency));
                                        }
                                        _ => {
                                            budgeted_amount.set(Money::zero(budget_signal().currency));
                                        }
                                    }
                                },
                            }
                            Slider {
                                value: SliderValue::Single(budgeted_amount().amount_in_dollars() as f64),
                                min: 0.0,
                                max: (budgeted_amount() + remaining_to_budget).amount_in_dollars() as f64,
                                step: 1.0,
                                label: "MONEEYYY",
                                horizontal: true,
                                on_value_change: move |v| {
                                    let SliderValue::Single(v) = v;
                                    budgeted_amount.set(Money::new_dollars(v as i64, budget_signal().currency));
                                },
                                SliderTrack {
                                    SliderRange {}
                                    SliderThumb {}
                                }
                            }
                        }
                        div { class: "budget-item-edit-actions",
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| async move {
                                    if item.actual_id.is_none() {
                                        match api::add_actual(
                                                budget_id,
                                                item.item_id,
                                                budgeted_amount(),
                                                budget_signal().period_id,
                                            )
                                            .await
                                        {
                                            Ok(updated_budget) => {
                                                consume_context::<BudgetState>().0.set(updated_budget);
                                                edit_item.set(false)
                                            }
                                            Err(_) => {
                                                edit_item.set(false);
                                            }
                                        }
                                    } else {
                                        match api::modify_actual(
                                                budget_id,
                                                item.actual_id.unwrap(),
                                                budget_signal().period_id,
                                                Some(budgeted_amount()),
                                                None,
                                            )
                                            .await
                                        {
                                            Ok(updated_budget) => {
                                                consume_context::<BudgetState>().0.set(updated_budget);
                                                edit_item.set(false)
                                            }
                                            Err(_) => {
                                                edit_item.set(false);
                                            }
                                        }
                                    }
                                },
                                "Spara"
                            }
                            Button {
                                variant: ButtonVariant::Secondary,
                                onclick: move |_| {
                                    edit_item.set(false);
                                },
                                "Avbryt"
                            }
                        }
                    }
                }
            }
        } else {
            rsx! {
                div { class: "budget-item", key: "{item.item_id}",
                    div {
                        class: "budget-item-name",
                        onclick: move |_| { expanded.set(!expanded()) },
                        "{item.name}"
                    }
                    Button {
                        variant: ButtonVariant::Ghost,
                        onclick: move |_| { edit_item.set(true) },
                        Pen {}
                    }
                    BudgetItemStatusView { item: item.clone() }
                    div { class: "budget-item-amounts",
                        "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    }
                }
            }
        }   
    }
