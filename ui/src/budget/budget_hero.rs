use crate::{BudgetingTypeTabs, Input};
use api::models::Budget;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use crate::file_chooser::*;

const HERO_CSS: Asset = asset!("assets/styling/budget-hero.css");
#[component]
pub fn BudgetHero() -> Element {
    let budget_resource = use_server_future(api::get_default_budget)?;

    let mut budget_signal = use_signal(|| None::<Budget>);
    
    use_context_provider(|| budget_signal);
    
    let mut budget_name = use_signal(|| "".to_string());

    use_effect(move || {
        if let Some(Ok(Some(budget))) = budget_resource.read().as_ref() {
            tracing::info!("We have budget: {}", budget.id);
            budget_signal.set(Some(budget.clone()));
        }
    });
    
    let mut file_chosen = use_signal(|| None::<String>);
    
    use_effect(move || {
        if let Some(file_name) = file_chosen() {
            tracing::info!("File chosen: {}", file_name);
            file_chosen.set(None);
        }
    });

    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            tracing::info!("The budget signal was updated: {}", budget.id);
            rsx! {
                document::Link { rel: "stylesheet", href: HERO_CSS }
                div { class: "budget-hero-container",
                    // Header
                    div { class: "budget-header",
                        h1 { class: "budget-title", {budget.name.clone()} }
                        FileDialog {
                            on_chosen: move |event| {
                                tracing::info!("File chosen: {}",event.data);
                                file_chosen.set(Some(event.data));
                            },
                        }
                    }
                    div { class: "budget-hero-content",
                        BudgetingTypeTabs {
                            budget_id: budget.id,
                            items_by_type: budget.items_by_type(),
                        }
                    }
                }
            }
        }
        None => {
            // Check if we have an error or are still loading
            match budget_resource.read_unchecked().as_ref() {
                Some(Err(e)) => rsx! {
                    document::Link { rel: "stylesheet", href: HERO_CSS }
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
                    document::Link { rel: "stylesheet", href: HERO_CSS }

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
                                oninput: move |e: FormEvent| { budget_name.set(e.value()) },
                            }
                        }
                    }
                    br {}
                    button {
                        class: "button",
                        "data-style": "primary",
                        onclick: move |_| async move {
                            if let Ok(budget) = api::create_budget(budget_name.to_string(), None).await {
                                tracing::info!("UI received a budget: {budget:?}");
                                budget_signal.set(Some(budget));
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
