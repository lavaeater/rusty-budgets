use api::models::{Budget, BudgetActionOverview};
use dioxus::logger::tracing;
use dioxus::prelude::*;
use lucide_dioxus::{Hamburger, Pen};
use uuid::Uuid;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

pub static CURRENT_BUDGET_ID: GlobalSignal<Uuid> = Signal::global(|| Uuid::default());

#[component]
pub fn BudgetHero() -> Element {
    let mut budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetActionOverview>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    // Handle the resource state
    match budget_signal() {
        Some(mut budget) => {
            
            rsx! {
            document::Link { rel: "stylesheet", href: BUDGET_CSS }
            div {
                class: "budget-hero-container",
                
                // Header Section
                div {
                    class: "budget-header",
                    h1 { class: "budget-title", "{budget.name}" }
                    div {
                        class: "budget-stats",
                        div {
                            class: "stat-card income",
                            div { class: "stat-label", "💰 Unallocated Income" }
                            div { class: "stat-value", "${budget.total_unallocated_income:.2}" }
                        }
                        div {
                            class: "stat-card expense",
                            div { class: "stat-label", "⚠️ Overdrawn" }
                            div { class: "stat-value", "${budget.total_overdrawn_amount:.2}" }
                        }
                        div {
                            class: "stat-card total",
                            div { class: "stat-label", "📊 Action Items" }
                            div { class: "stat-value", "{budget.action_items.len()}" }
                        }
                    }
                }
                
                // Action Items Section
                if !budget.action_items.is_empty() {
                    div {
                        class: "action-items-section",
                        h2 { class: "section-title", "🎯 Items Requiring Attention" }
                        div {
                            class: "action-items-grid",
                            for action_item in &budget.action_items {
                                div {
                                    class: match action_item.issue_type {
                                        api::models::ActionItemType::UnallocatedIncome => "action-item income-item",
                                        api::models::ActionItemType::OverdrawnExpense => "action-item expense-item",
                                    },
                                    div {
                                        class: "action-item-header",
                                        div {
                                            class: "action-item-icon",
                                            match action_item.issue_type {
                                                api::models::ActionItemType::UnallocatedIncome => "💵",
                                                api::models::ActionItemType::OverdrawnExpense => "🚨",
                                            }
                                        }
                                        div {
                                            class: "action-item-title",
                                            h3 { "{action_item.name}" }
                                            span { class: "category-tag", "{action_item.category}" }
                                        }
                                        div {
                                            class: "action-item-amount",
                                            "${action_item.amount:.2}"
                                        }
                                    }
                                    div {
                                        class: "action-item-description",
                                        p { "{action_item.description}" }
                                    }
                                    div {
                                        class: "action-item-actions",
                                        button {
                                            class: "btn-primary",
                                            onclick: move |_| {
                                                // TODO: Implement action handling
                                                tracing::info!("Action clicked for item!");
                                            },
                                            match action_item.issue_type {
                                                api::models::ActionItemType::UnallocatedIncome => "Allocate Funds",
                                                api::models::ActionItemType::OverdrawnExpense => "Review Spending",
                                            }
                                        }
                                        button {
                                            class: "btn-secondary",
                                            onclick: move |_| {
                                                tracing::info!("Details clicked for item!");
                                            },
                                            "View Details"
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div {
                        class: "no-action-items",
                        div { class: "success-icon", "✅" }
                        h2 { "All Good!" }
                        p { "Your budget is balanced and no items require immediate attention." }
                    }
                }
                
                // Quick Actions Section
                div {
                    class: "quick-actions-section",
                    h2 { class: "section-title", "⚡ Quick Actions" }
                    div {
                        class: "quick-actions-grid",
                        button {
                            class: "quick-action-btn",
                            onclick: move |_| {
                                tracing::info!("Add income clicked");
                            },
                            div { class: "quick-action-icon", "💰" }
                            span { "Add Income" }
                        }
                        button {
                            class: "quick-action-btn",
                            onclick: move |_| {
                                tracing::info!("Add expense clicked");
                            },
                            div { class: "quick-action-icon", "💸" }
                            span { "Add Expense" }
                        }
                        button {
                            class: "quick-action-btn",
                            onclick: move |_| {
                                tracing::info!("View full budget clicked");
                            },
                            div { class: "quick-action-icon", "📊" }
                            span { "Full Budget" }
                        }
                        button {
                            class: "quick-action-btn",
                            onclick: move |_| {
                                budget_resource.restart();
                            },
                            div { class: "quick-action-icon", "🔄" }
                            span { "Refresh" }
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
