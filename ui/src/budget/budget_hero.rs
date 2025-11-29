use api::models::*;
use crate::budget::{TransactionsView, BudgetTabs};
use crate::file_chooser::*;
use crate::{Button, Input};
use api::{get_budget, import_transactions};
use chrono::Utc;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use std::future::Future;
use uuid::Uuid;
use api::view_models::*;

const HERO_CSS: Asset = asset!("assets/styling/budget-hero-a.css");
#[component]
pub fn BudgetHero() -> Element {
    let mut period_id = use_signal(|| PeriodId::from_date(Utc::now(), MonthBeginsOn::default()));
    let budget_resource = use_server_future(move || get_budget(None, period_id()))?;


    let mut budget_signal = use_signal(|| None::<BudgetViewModel>);
    use_context_provider(|| budget_signal);

    let mut budget_name = use_signal(|| "".to_string());
    let mut budget_id = use_signal(|| Uuid::default());

    use_effect(move || {
        if let Some(Ok(Some(budget))) = budget_resource.read().as_ref() {
            info!("We have budget: {}", budget.id);
            budget_signal.set(Some(budget.clone()));
            budget_name.set(budget.name.clone());
        }
    });

    let import_file = move |file: FileChosen| {
        let file_name = file.data.to_string();
        spawn(async move {
            if !file_name.is_empty() {
                if let Ok(updated_budget) = import_transactions(budget_id(), file_name, period_id()).await {
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

            rsx! {
                document::Link { rel: "stylesheet", href: HERO_CSS }
                div { class: "budget-hero-a-container",
                    // Header with quick stats
                    div { class: "budget-header-a",
                        div { class: "header-title",
                            h1 { {budget.name.clone()} }
                            h2 { {period_id().to_string()} }
                            Button {
                                onclick: move |_| {
                                    period_id.set(period_id().month_before());
                                },
                                "Previous period"
                            }
                            Button {
                                onclick: move |_| {
                                    period_id.set(period_id().month_after());
                                },
                                "Next period"
                            }
                        }
                        div { class: "header-actions",
                            FileDialog { on_chosen: import_file }
                            if !budget.to_connect.is_empty() {
                                div { class: "unassigned-badge",
                                    "{budget.to_connect.len()} transaktioner att hantera"
                                }
                            }
                            if !budget.ignored_transactions.is_empty() {
                                div { class: "unassigned-badge",
                                    "{budget.ignored_transactions.len()} transaktioner att hantera"
                                }
                            }
                        }
                    }
                    // Dashboard cards showing overview
                    div { class: "dashboard-cards",
                        for overview in budget.overviews {
                            div { class: format!("overview-card {}", if !overview.is_ok { "over-budget" } else { "" }),
                                h3 { class: if !overview.is_ok { "warning" } else { "" },
                                    {overview.budgeting_type.to_string()}
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
                    if budget.to_connect.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Alla transaktioner är hanterade!" }
                        }
                    } else {
                        div { class: "transactions-section-prominent",
                            "TransactionsView"
                            TransactionsView { ignored: false }
                        }
                    }
                    if budget.ignored_transactions.is_empty() {
                        div { class: "transactions-section-minimal",
                            p { class: "success-message", "✓ Inga ignorerade transaktioner!" }
                        }
                    } else {
                        div { class: "transactions-section-prominent",
                            "Another transactions view"
                            TransactionsView { ignored: true }
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
                            if let Ok(budget) = api::create_budget(
                                    budget_name.to_string(),
                                    period_id(),
                                    Some(true),
                                )
                                .await
                            {
                                info!("UI received a budget: {budget:?}");
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
