use crate::budget::budget_hero::BudgetState;
use api::models::BudgetingType;
use api::view_models::{BudgetItemStatus, BudgetItemViewModel, BudgetViewModel};
use dioxus::prelude::*;

#[component]
pub fn BudgetItemStatusView(item: BudgetItemViewModel) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    let budget_id = budget.id;
    let mut item_status = use_signal(|| item.status);
    match item_status() {
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
                                        item_status.set(BudgetItemStatus::Balanced);
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
                                item_status.set(BudgetItemStatus::Balanced);
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
