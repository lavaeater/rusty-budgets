use uuid::Uuid;
use dioxus::prelude::*;
use api::models::{Budget, BudgetingType, Currency, Money};
use api::view_models::BudgetViewModel;
use crate::components::{Input, Button};

#[component]
pub fn NewBudgetItem(budgeting_type: BudgetingType, tx_id: Option<Uuid>, close_signal: Option<Signal<bool>>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<BudgetViewModel>>>();
    let new_item_name = use_signal(|| "".to_string());
    let new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));
    
    match budget_signal() {
        Some(budget) => {
            let budget_id = budget.id;
            let period_id = budget.period_id;

            rsx! {}
        }
        None => {
            rsx! {
                div { "No budget" }
            }
        }
    }
}