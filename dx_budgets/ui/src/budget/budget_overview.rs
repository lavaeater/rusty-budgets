use dioxus::dioxus_core::Element;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget::budget_hero::{BudgetSignal, DEFAULT_BUDGET_ID};
use api::BudgetOverview as Budget;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

#[component]
pub fn BudgetOverview(id: Uuid) -> Element {
    // let id = id.clone();
    let mut budget_resource = 
        use_resource(move || async move { api::get_budget_overview(id).await });

    // Persistent signal for budget data
    let mut budget_signal = use_signal(|| None::<Budget>);
    // Update budget signal when resource changes
    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            let sett = budget.clone();
            *budget_signal.write() = Some(sett);
        }
    });
    
    match budget_signal() {
        Some(budget) => {
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div {
                    id: "budget_overview",
                        h1 {
                            "{budget.name}"
                        }
                        h4 {
                            "Default: {budget.default_budget}"
                        }
                }
                for item in budget.incomes {
                    h3 {
                        "{item.name}"
                    }
                    if item.money_needs_job {
                        h4 {
                            "Money left to spend: {item.aggregate_amount}"
                        }
                    }
                    if item.too_much_job {
                        h4 {
                            "Money over budget: {item.aggregate_amount}"
                        }
                    }
                    if item.is_balanced {
                        h4 {
                            "There is balance to the force"
                        }
                    }
                }
                for item in budget.expenses {
                    h3 {
                        "{item.name}"
                    }
                    if item.money_needs_job {
                        h4 {
                            "Money left to spend: {item.aggregate_amount}"
                        }
                    }
                    if item.too_much_job {
                        h4 {
                            "Money over budget: {item.aggregate_amount}"
                        }
                    }
                    if item.is_balanced {
                        h4 {
                            "There is balance to the force"
                        }
                    }
                }
                for item in budget.savings {
                    h3 {
                        "{item.name}"
                    }
                    if item.money_needs_job {
                        h4 {
                            "Money left to spend: {item.aggregate_amount}"
                        }
                    }
                    if item.too_much_job {
                        h4 {
                            "Money over budget: {item.aggregate_amount}"
                        }
                    }
                    if item.is_balanced {
                        h4 {
                            "There is balance to the force"
                        }
                    }
                }
            }
        }
        None => {
            // Check if we have an error or are still loading
            match budget_resource.read_unchecked().as_ref() {
                Some(Err(e)) => rsx! {
                    div {
                        id: "budget_overview",
                        h4 { "Error loading budget: {e}" }
                    }
                },
                _ => rsx! {
                    div {
                        id: "budget_overview",
                        h4 { "Loading..." }
                    }
                },
            }
        }
    }
}