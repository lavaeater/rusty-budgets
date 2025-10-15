use crate::file_chooser::*;
use crate::budget_b::{BudgetTabs, TransactionsView};
use crate::Input;
use api::models::Budget;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use std::future::Future;
use uuid::Uuid;

const HERO_CSS: Asset = asset!("assets/styling/budget-hero-b.css");
#[component]
pub fn BudgetHero() -> Element {
    let budget_resource = use_server_future(api::get_default_budget)?;

    let mut budget_signal = use_signal(|| None::<Budget>);
    let mut budget_id = use_signal(Uuid::default);

    use_context_provider(|| budget_signal);

    let mut budget_name = use_signal(|| "".to_string());

    use_effect(move || {
        if let Some(Ok(Some(budget))) = budget_resource.read().as_ref() {
            tracing::info!("We have budget: {}", budget.id);
            budget_signal.set(Some(budget.clone()));
        }
    });

    let import_file = move |file: FileChosen| {
        let file_name = file.data.to_string();
        spawn(async move {
            if !file_name.is_empty() {
                if let Ok(updated_budget) = api::import_transactions(budget_id(), file_name).await {
                    budget_signal.set(Some(updated_budget));
                }
            }
        });
    };

    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            tracing::info!("The budget signal was updated: {}", budget.id);
            budget_id.set(budget.id);
            let unassigned_transactions = budget.list_transactions_for_connection();
            let has_unassigned = !unassigned_transactions.is_empty();
            
            rsx! {
                document::Link { rel: "stylesheet", href: HERO_CSS }
                div { class: "budget-hero-b-container",
                    // Top header bar
                    div { class: "budget-header-b",
                        div { class: "header-content",
                            h1 { {budget.name.clone()} }
                            span { class: "period-badge", {budget.get_current_period_id().to_string()} }
                        }
                        FileDialog { on_chosen: import_file }
                    }
                    
                    // Main layout: sidebar + content
                    div { class: "budget-layout-b",
                        // Left sidebar for transactions (workflow)
                        if has_unassigned {
                            div { class: "transactions-sidebar",
                                div { class: "sidebar-header",
                                    h2 { "Att hantera" }
                                    span { class: "sidebar-count", "{unassigned_transactions.len()}" }
                                }
                                TransactionsView {
                                    budget_id: budget.id,
                                    transactions: unassigned_transactions,
                                    items: budget.list_all_items(),
                                }
                            }
                        }
                        
                        // Main content area
                        div { 
                            class: if has_unassigned { "budget-content-with-sidebar" } else { "budget-content-full" },
                            BudgetTabs {}
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
