use api::models::*;
use dioxus::prelude::*;
use uuid::Uuid;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

pub static CURRENT_BUDGET_ID: GlobalSignal<Uuid> = Signal::global(|| Uuid::default());

#[component]
pub fn BudgetHero() -> Element {
    let budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetSummary>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            rsx! {
            document::Link { rel: "stylesheet", href: BUDGET_CSS }
            div {
                id: "budget_hero",
                    span {
                        h2 {"{budget.name}"},
                        }
                    }
                }
        }
        None => {
            // Check if we have an error or are still loading
            match budget_resource.read_unchecked().as_ref() {
                Some(Err(e)) => rsx! {
                    div {
                        id: "budget_hero",
                        h4 { "Error loading budget: {e}" }
                    }
                },
                _ => rsx! {
                    div {
                        id: "budget_hero",
                        h4 { "Loading..." }
                    }
                },
            }
        }
    }
}
