use crate::budget::budget_hero::BudgetState;
use api::models::BudgetingType;
use api::view_models::{BudgetItemStatus, BudgetItemViewModel};
use dioxus::prelude::*;
use uuid::Uuid;

#[component]
pub fn BudgetItemStatusView(item: BudgetItemViewModel) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    let budget_id = budget.id;
    let mut item_status = use_signal(|| item.status);
    let mut show_take_from = use_signal(|| false);

    match item_status() {
        BudgetItemStatus::Balanced => {
            rsx!()
        }
        BudgetItemStatus::OverBudget => {
            let shortage = item.actual_amount - item.budgeted_amount;

            // Items that can donate: Expense or Savings, not this item, with remaining_budget >= shortage
            let can_donate: Vec<_> = budget
                .items
                .iter()
                .filter(|other| {
                    other.item_id != item.item_id
                        && matches!(
                            other.budgeting_type,
                            BudgetingType::Expense | BudgetingType::Savings
                        )
                        && other.actual_id.is_some()
                        && other.remaining_budget >= shortage
                })
                .cloned()
                .collect();

            rsx! {
                span { class: "over-budget-indicator", "Över budget" }
                button {
                    class: "auto-adjust-button",
                    onclick: move |_| async move {
                        let actual_id = item.actual_id.unwrap();
                        match api::modify_actual(
                                budget_id,
                                actual_id,
                                budget.period_id,
                                Some(item.actual_amount),
                                None,
                            )
                            .await
                        {
                            Ok(updated_budget) => {
                                consume_context::<BudgetState>().0.set(updated_budget);
                                item_status.set(BudgetItemStatus::Balanced);
                            }
                            Err(e) => {
                                error!("Failed to adjust item funds: {}", e);
                            }
                        }
                    },
                    "Auto-justera (+{shortage})"
                }
                if !can_donate.is_empty() {
                    button {
                        class: "auto-adjust-button",
                        onclick: move |_| show_take_from.set(!show_take_from()),
                        if show_take_from() { "Avbryt" } else { "Ta från..." }
                    }
                    if show_take_from() {
                        div { class: "take-from-picker",
                            span { class: "take-from-label", "Flytta {shortage} från:" }
                            for donor in &can_donate {
                                {
                                    let donor_actual_id = donor.actual_id.unwrap();
                                    let to_actual_id = item.actual_id.unwrap();
                                    let period_id = budget.period_id;
                                    let donor_name = donor.name.clone();
                                    let donor_remaining = donor.remaining_budget;
                                    rsx! {
                                        button {
                                            class: "take-from-option",
                                            key: "{donor_actual_id}",
                                            onclick: move |_| async move {
                                                match api::reallocate_funds(
                                                        budget_id,
                                                        period_id,
                                                        donor_actual_id,
                                                        to_actual_id,
                                                        shortage,
                                                    )
                                                    .await
                                                {
                                                    Ok(updated_budget) => {
                                                        consume_context::<BudgetState>().0.set(updated_budget);
                                                        show_take_from.set(false);
                                                        item_status.set(BudgetItemStatus::Balanced);
                                                    }
                                                    Err(e) => {
                                                        error!("Failed to reallocate funds: {}", e);
                                                    }
                                                }
                                            },
                                            "{donor_name}"
                                            span { class: "take-from-remaining", " ({donor_remaining} kvar)" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        BudgetItemStatus::NotBudgeted => {
            rsx! {
                span { class: "over-budget-indicator", "Ej budgeterad" }
            }
        }
        BudgetItemStatus::UnderBudget => {
            let shortage = item.budgeted_amount - item.actual_amount;
            rsx! {
                span { class: "over-budget-indicator", "Under budget" }
                button {
                    class: "auto-adjust-button",
                    onclick: move |_| async move {
                        let actual_id = item.actual_id.unwrap();
                        match api::modify_actual(
                                budget_id,
                                actual_id,
                                budget.period_id,
                                Some(item.actual_amount),
                                None,
                            )
                            .await
                        {
                            Ok(updated_budget) => {
                                consume_context::<BudgetState>().0.set(updated_budget);
                                item_status.set(BudgetItemStatus::Balanced);
                            }
                            Err(e) => {
                                error!("Failed to adjust item funds: {}", e);
                            }
                        }
                    },
                    "Auto-justera (-{shortage})"
                }
            }
        }
    }
}
