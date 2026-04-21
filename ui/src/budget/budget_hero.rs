use crate::Button;
use crate::budget::{BudgetTabs, CreateBudgetItemsView, RetagTransactionsView, RulesView, TagTransactionsView, TagsView, TransactionsView, TransferPairsView};
use crate::file_chooser::{FileData, FileDialog};
use api::models::*;
use api::view_models::BudgetViewModel;
use api::view_models::*;
use api::{auto_budget_period, get_budget, import_transactions_bytes};
use chrono::Utc;
use dioxus::core::internal::generational_box::GenerationalRef;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
use std::cell::Ref;
use std::future::Future;
use uuid::Uuid;

#[derive(Clone, Copy)]
pub struct BudgetState(pub Signal<BudgetViewModel>);

#[derive(Clone, Copy)]
pub enum BudgetLoadingState {
    Loading,
    Loaded,
    Error,
    NoDefaultBudget,
}

const HERO_CSS: Asset = asset!("assets/styling/budget-hero.css");
#[component]
pub fn BudgetHero() -> Element {
    let mut budget_loading_state = use_signal(|| BudgetLoadingState::Loading);
    let budget_id = use_signal(Uuid::default);
    let period_id = use_signal(|| PeriodId::from_date(Utc::now(), MonthBeginsOn::default()));

    let budget_resource = use_server_future(move || get_budget(None, period_id()))?;

    let mut budget_name = use_signal(|| "".to_string());
    let state_signal = use_signal(BudgetViewModel::default);
    use_context_provider(|| BudgetState(state_signal));

    use_effect(move || match budget_resource.read().as_ref() {
        None => {
            info!("Resoure result is None");
            budget_loading_state.set(BudgetLoadingState::Loading);
        }
        Some(resource_result) => {
            info!("Resoure result is Some");
            match resource_result {
                Ok(viewmodel_result) => {
                    info!("Resoure result is Ok");
                    match viewmodel_result {
                        None => {
                            info!("Viewmodel result is None");
                            budget_loading_state.set(BudgetLoadingState::NoDefaultBudget);
                        }
                        Some(budget_viewmodel) => {
                            info!("Viewmodel result is Some");
                            budget_loading_state.set(BudgetLoadingState::Loaded);
                            consume_context::<BudgetState>()
                                .0
                                .set(budget_viewmodel.clone());
                            budget_name.set(budget_viewmodel.name.clone());
                            info!("We have budget: {}", budget_viewmodel.id);
                        }
                    }
                }
                Err(err) => {
                    error!(error = %err, "Failed to get budget");
                    budget_loading_state.set(BudgetLoadingState::Error);
                }
            }
        }
    });

    match budget_loading_state() {
        BudgetLoadingState::Loading => {
            rsx! {
                div { id: "budget_hero",
                    h4 { "Laddar..." }
                }
            }
        }
        BudgetLoadingState::Loaded => {
            rsx! {
                BudgetOverview { budget_id, period_id }
            }
        }
        BudgetLoadingState::Error => {
            rsx! {
                div { id: "budget_hero",
                    h4 { "Något gick fel vid inläsning av budget." }
                }
            }
        }
        BudgetLoadingState::NoDefaultBudget => {
            rsx! {
                document::Link { rel: "stylesheet", href: HERO_CSS }

                div { display: "flex", flex_direction: "column", gap: ".5rem",
                    h4 { "Ingen budget hittad" }
                    Label { html_for: "name", "Skapa budget" }
                    div {
                        display: "flex",
                        flex_direction: "column",
                        width: "40%",
                        input {
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
                            info!("Created new budget: {budget:?}");
                            budget_loading_state.set(BudgetLoadingState::Loaded);
                            consume_context::<BudgetState>().0.set(budget.clone());
                            budget_name.set(budget.name.clone());
                        }
                    },
                    "Skapa budget"
                }
            }
        }
    }
}

#[component]
pub fn BudgetOverview(mut budget_id: Signal<Uuid>, mut period_id: Signal<PeriodId>) -> Element {
    let period_id_now = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    info!("The budget signal was updated: {}", budget.id);
    budget_id.set(budget.id);

    let auto_budget_enabled = budget.period_id != period_id_now;
    let import_file = move |file: FileData| {
        let contents = file.contents;
        spawn(async move {
            if !contents.is_empty()
                && let Ok(updated_budget) =
                    import_transactions_bytes(budget_id(), contents, period_id()).await
            {
                info!("Import went well and we update the context bro");
                consume_context::<BudgetState>().0.set(updated_budget);
            }
        });
    };
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
                                if let Ok(bv) = auto_budget_period(budget_id(), period_id()).await {
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
                        // Main content area with tabs
                        // Transactions section - prominent if there are unassigned
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
            div { class: "budget-main-content", BudgetTabs {} }
            if budget.untagged_transaction_count > 0 {
                div { class: "transactions-section-prominent",
                    h3 { style: "margin: 0 0 16px 0;",
                        "{budget.untagged_transaction_count} transaktioner att tagga"
                    }
                    TagTransactionsView {}
                }
            }
            if !budget.potential_transfers.is_empty() {
                div { class: "transactions-section-prominent", TransferPairsView {} }
            }
            if budget.untagged_transaction_count == 0 && budget.potential_transfer_count == 0 {
                div { class: "transactions-section-prominent",
                    h3 { style: "margin: 0 0 16px 0;", "Skapa budgetposter" }
                    CreateBudgetItemsView {}
                }
            }
            div { class: "transactions-section-prominent",
                h3 { style: "margin: 0 0 16px 0;", "Taggade transaktioner" }
                RetagTransactionsView {}
            }
            div { class: "transactions-section-prominent",
                h3 { style: "margin: 0 0 16px 0;", "Taggar" }
                TagsView {}
            }
            div { class: "transactions-section-prominent",
                h3 { style: "margin: 0 0 16px 0;", "Taggningsregler" }
                RulesView {}
            }
            if budget.to_connect.is_empty() {
                div { class: "transactions-section-minimal",
                    p { class: "success-message", "✓ Alla transaktioner är hanterade!" }
                }
            } else {
                div { class: "transactions-section-prominent",
                    TransactionsView { ignored: false }
                }
            }
            if budget.ignored_transactions.is_empty() {
                div { class: "transactions-section-minimal",
                    p { class: "success-message", "✓ Inga ignorerade transaktioner!" }
                }
            } else {
                div { class: "transactions-section-prominent",
                    TransactionsView { ignored: true }
                }
            }
        }
    }
}
