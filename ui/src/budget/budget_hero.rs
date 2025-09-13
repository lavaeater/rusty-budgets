use dioxus_primitives::label::Label;
use api::cqrs::budget::Budget;
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget::budget_groups::BudgetGroups;
use crate::Input;

pub static CURRENT_BUDGET_ID: GlobalSignal<Uuid> = Signal::global(|| Uuid::default());
#[component]
pub fn BudgetHeroOne() -> Element {
    let budget_resource = use_server_future(api::get_default_budget)?;
    let mut budget_signal = use_signal(|| None::<Budget>);
    let mut budget_name = use_signal(|| "".to_string());

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
                div { class: "budget-hero-container",
                    // Header
                    div { class: "budget-header",
                        h1 { class: "budget-title", {budget.name} }
                    }
                    BudgetGroups { groups: budget.budget_groups.values().cloned().collect() }
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
                        h4 { "Laddar..." }
                    }
                },
                Some(&Ok(None)) => rsx! {
                    div {
                        display: "flex",
                        flex_direction: "column",
                        gap: ".5rem",
                        h4 { "Ingen budget hittad" }
                        Label { html_for: "name", "Skapa budget" }
                        div {
                            display: "flex",
                            flex_direction: "column",
                            width: "40%",
                            Input {
                                id: "name",
                                placeholder: "Budgetnamn",
                                oninput: move |e: FormEvent| { 
                                    
                                    budget_name.set(e.value())
                                },
                            }
                        }
                    }
                    br {}
                    button {
                        class: "button",
                        "data-style": "primary",
                        onclick: move |_| async move {
                            if let Ok(budget) = api::create_budget(budget_name.to_string(), None).await {
                                budget_signal.set(Some(budget))
                            }
                        },
                        "Skapa budget"
                    }
                },
                Some(&Ok(Some(_))) => rsx! {
                    div { id: "budget_hero",
                        h4 { "Laddar..." }
                    }
                },
            }
        }
    }
}
