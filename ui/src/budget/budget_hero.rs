use crate::file_chooser::*;
use crate::budget::{BudgetTabs, TransactionsView};
use crate::{Button, Input};
use api::models::{Budget, PeriodId};
use api::{import_transactions, get_budget};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use std::future::Future;
use uuid::Uuid;
use chrono::Utc;

const HERO_CSS: Asset = asset!("assets/styling/budget-hero-a.css");
#[component]
pub fn BudgetHero() -> Element {
    let budget_resource = use_server_future(get_budget(None))?;
    let mut period_id = use_signal(|| PeriodId::default());

    let mut budget_signal = use_signal(|| None::<Budget>);
    let mut budget_id = use_signal(Uuid::default);

    use_context_provider(|| budget_signal);
    use_context_provider(|| period_id);

    let mut budget_name = use_signal(|| "".to_string());

    use_effect(move || {
        if let Some(Ok(Some(budget))) = budget_resource.read().as_ref() {
            info!("We have budget: {}", budget.id);
            budget_signal.set(Some(budget.clone()));
            period_id.set(PeriodId::from_date(Utc::now(), budget.month_begins_on()));
        }
    });

    let import_file = move |file: FileChosen| {
        let file_name = file.data.to_string();
        spawn(async move {
            if !file_name.is_empty() {
                if let Ok(updated_budget) = import_transactions(budget_id(), file_name).await {
                    budget_signal.set(Some(updated_budget));
                }
            }
        });
    };
    
    // Handle the resource state
    match budget_signal() {
        Some(budget) => {
            
            info!("The budget signal was updated: {}", budget.id);
            budget_id.set(budget.id);
            let transactions_for_connection = budget.list_transactions_for_connection(current_period_id());
            let ignored_transactions = budget.list_ignored_transactions(current_period_id());
            let items_by_type = budget.items_by_type(current_period_id());
            
            rsx! {
                document::Link { rel: "stylesheet", href: HERO_CSS }
                div { class: "budget-hero-a-container",
                    // Header with quick stats
                    div { class: "budget-header-a",
                        div { class: "header-title",
                            h1 { {budget.name.clone()} }
                            h2 { {current_period_id().unwrap_or(PeriodId::default()).to_string()} }
                            Button {
                                onclick: move |_| {
                                    if let Some(period_id) = current_period_id() {
                                        current_period_id.set(Some(period_id.month_before()));
                                    }
                                },
                                "Previous period"
                            }
                            Button {
                                onclick: move |_| {
                                    if let Some(period_id) = current_period_id() {
                                        current_period_id.set(Some(period_id.month_after()));
                                    }
                                },
                                "Next period"
                            }
                        }
                        div { class: "header-actions",
                            FileDialog { on_chosen: import_file }
                            if !transactions_for_connection.is_empty() {
                                div { class: "unassigned-badge",
                                    "{transactions_for_connection.len()} transaktioner att hantera"
                                }
                            }
                            if !ignored_transactions.is_empty() {
                                div { class: "unassigned-badge",
                                    "{ignored_transactions.len()} transaktioner att hantera"
                                }
                            }
                        }
                    }
                    // Dashboard cards showing overview
                    div { class: "dashboard-cards",
                        for (_ , budgeting_type , overview , _) in &items_by_type {
                            div { class: format!("overview-card {}", if !overview.is_ok { "over-budget" } else { "" }),
                                h3 { class: if !overview.is_ok { "warning" } else { "" },
                                    {budgeting_type.to_string()}
                                }
                                div { class: "card-stats",
                                    div { class: "stat",
                                        span { class: "stat-label", "Budgeterat" }
                                        span { class: "stat-value",
                                            {overview.budgeted_amount.to_string()}
                                        }
                                    }
                                    div { class: "stat",
                                        span { class: "stat-label", "Faktiskt" }
                                        span { class: "stat-value",
                                            {overview.actual_amount.to_string()}
                                        }
                                    }
                                    div { class: "stat",
                                        span { class: "stat-label", "Återstår" }
                                        span { class: "stat-value stat-remaining",
                                            {overview.remaining_budget.to_string()}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // Main content area with tabs
                    div { class: "budget-main-content", BudgetTabs {} }
                    // Transactions section - prominent if there are unassigned
                    if transactions_for_connection.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Alla transaktioner är hanterade!" }
                        }
                    } else {
                        div { class: "transactions-section-prominent",
                            TransactionsView {
                                budget_id: budget_id(),
                                transactions: transactions_for_connection,
                                items: budget.list_all_items(),
                            }
                        }
                    }
                    if ignored_transactions.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Inga ignorerade transaktioner!" }
                        }
                    } else {
                        div { class: "transactions-section-prominent",
                            TransactionsView {
                                budget_id: budget.id,
                                transactions: ignored_transactions,
                                items: budget.list_all_items(),
                            }
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
