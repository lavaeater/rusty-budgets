use api::models::*;
use crate::budget::{BudgetTabs, TransactionsView};
use crate::file_chooser::*;
use crate::{Button, Input};
use api::{auto_budget_period, get_budget, import_transactions};
use chrono::Utc;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use std::future::Future;
use uuid::Uuid;
use api::view_models::*;
use api::view_models::BudgetViewModel;

#[derive(Clone, Copy)]
pub struct BudgetState(pub Signal<BudgetViewModel>);

const HERO_CSS: Asset = asset!("assets/styling/budget-hero.css");
#[component]
pub fn BudgetHero() -> Element {
    let mut period_id = use_signal(|| PeriodId::from_date(Utc::now(), MonthBeginsOn::default()));
    let budget_resource = use_server_future(move || get_budget(None, period_id()))?;
    
    let period_id_now = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());
    
    let mut budget_name = use_signal(|| "".to_string());
    let mut budget_id = use_signal(|| Uuid::default());
    let mut ready = use_signal(|| false);
    let mut budget_signal = use_signal(|| BudgetViewModel::default());
    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            info!("We have budget: {}", budget.id); 
            budget_signal.set(budget.clone());
            use_context_provider(|| BudgetState(budget_signal));
            budget_name.set(budget.name.clone());
            ready.set(true);
        }
    });

    let import_file = move |file: FileChosen| {
        let file_name = file.data.to_string();
        spawn(async move {
            if !file_name.is_empty() {
                if let Ok(updated_budget) = import_transactions(budget_id(), file_name, period_id()).await {
                    consume_context::<BudgetState>().0.set(updated_budget);
                }
            }
        });
    };

    // Handle the resource state
    if ready() {
        let budget = use_context::<BudgetState>().0();
        info!("The budget signal was updated: {}", budget.id);
        budget_id.set(budget.id);
        
        let auto_budget_enabled = budget.period_id != period_id_now;

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
                                "Föregående period"
                            }
                            Button {
                                onclick: move |_| {
                                    period_id.set(period_id().month_after());
                                },
                                "Nästa period"
                            }
                            if auto_budget_enabled {
                                Button {
                                    onclick: move |_| async move {
                                        if let Ok(bv) = auto_budget_period(budget.id, period_id()).await {
                                            consume_context::<BudgetState>().0.set(bv);
                                        }
                                    },
                                    "Auto budget"
                                }
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
                                    "{budget.ignored_transactions.len()} ignorerade transaktioner"
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
        } else {
            rsx! {
                div { id: "budget_hero",
                    h4 { "Laddar..." }
                }
            }
        }
}
