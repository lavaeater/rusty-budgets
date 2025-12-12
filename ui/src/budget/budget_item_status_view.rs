use dioxus::prelude::*;
use api::models::BudgetingType;
use api::view_models::{BudgetItemStatus, BudgetItemViewModel, BudgetViewModel};
use crate::budget::budget_hero::BudgetState;

#[component]
pub fn BudgetItemStatusView(item: BudgetItemViewModel) -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;
    match item.status {
        BudgetItemStatus::Balanced => {
            rsx!()
        }
        BudgetItemStatus::OverBudget => {
            rsx! {
                span { class: "over-budget-indicator", "Över Budget" }
                {
                    let shortage = item.actual_amount - item.budgeted_amount;
                    let can_auto_adjust = item.budgeting_type == BudgetingType::Income
                        || budget
                            .overviews
                            .iter()
                            .find(|o| o.budgeting_type == BudgetingType::Income)
                            .map(|o| o.remaining_budget >= shortage)
                            .unwrap_or(false);
                    rsx! {
                        button {
                            class: "auto-adjust-button",
                            disabled: !can_auto_adjust,
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
                                    }
                                    Err(e) => {
                                        error!("Failed to adjust item funds: {}", e);
                                    }
                                }
                            },
                            "Auto-justera budgetbelopp (+{shortage})"
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
            let can_auto_adjust = true;
            rsx! {
                span { class: "over-budget-indicator", "Under budget" }
                button {
                    class: "auto-adjust-button",
                    disabled: !can_auto_adjust,
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
                            }
                            Err(e) => {
                                error!("Failed to adjust item funds: {}", e);
                            }
                        }
                    },
                    "Auto-justera budgetbelopp (-{shortage})"
                }
            }
        }

    }
}