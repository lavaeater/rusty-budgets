use crate::budget_popover::BudgetPopover;
use api::cqrs::budget::Budget;
use api::models::*;
use dioxus::prelude::*;
use uuid::Uuid;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

pub static CURRENT_BUDGET_ID: GlobalSignal<Uuid> = Signal::global(|| Uuid::default());
#[component]
pub fn BudgetHeroOne() -> Element {
    let budget_resource = use_server_future(api::get_default_budget)?;
    let mut budget_signal = use_signal(|| None::<Budget>);

    use_effect(move || {
        if let Some(Ok(Some(budget))) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-container",
                    // Header
                    div { class: "budget-header",
                        h1 { class: "budget-title", {budget.name} }
                    }
                }
            }
        }
        None => {
            // Check if we have an error or are still loading
            match budget_resource.read_unchecked().as_ref() {
                Some(Err(e)) => rsx! {
                    div { id: "budget_hero",
                        h4 { "Error loading budget: {e}" }
                    }
                },
                None => rsx! {
                    div { id: "budget_hero",
                        h4 { "Loading..." }
                    }
                },
                Some(&Ok(None)) => rsx! {
                    div { id: "budget_hero",
                        h4 { "NO DEFAULT BUDGET MATE" }
                    }
                },
                Some(&Ok(Some(_))) => rsx! {
                    div { id: "budget_hero",
                        h4 { "Loading..." }
                    }
                },
            }
        }
    }
}
