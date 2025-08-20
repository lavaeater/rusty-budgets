use api::models::*;
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget_popover::BudgetPopover;

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
            // Calculate progress of budgeted vs income
            let income = budget.total_income;
            let budgeted = budget.total_budgeted;
            let spent = budget.total_spent;
            let unallocated = budget.unallocated_income;
            let progress = if income > 0.0 {
                (budgeted / income) * 100.0
            } else {
                0.0
            };
            let progress_clamped = progress.clamp(0.0, 100.0);
            let balance_class = if budget.is_balanced {
                "balanced"
            } else {
                "unbalanced"
            };
            let balance_text = if budget.is_balanced {
                "Balanced"
            } else {
                "Unbalanced"
            };
            let status_icon = if budget.is_balanced { "✔" } else { "⚠" };
            let unallocated_class = if unallocated > 0.0 { "positive" } else { "" };
            let default_class = if budget.default_budget {
                "good"
            } else {
                "warning"
            };
            let default_text = if budget.default_budget { "Yes" } else { "No" };
            let balance_value_class = if (income - spent) >= 0.0 {
                "positive"
            } else {
                "warning"
            };

            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-container",
                    // Header
                    div { class: "budget-header",
                        h1 { class: "budget-title", {budget.name} }
                        div { class: {format_args!("balance-status {}", balance_class)},
                            span { class: "status-icon", {status_icon} }
                            span { {balance_text} }
                        }
                    }

                    // Financial overview cards
                    div { class: "financial-overview",
                        div { class: "overview-card income-card",
                            div { class: "card-header", "Total Income" }
                            div { class: "card-amount", {format_args!("${:.2}", income)} }
                        }
                        div { class: "overview-card budgeted-card",
                            div { class: "card-header", "Total Budgeted" }
                            div { class: "card-amount", {format_args!("${:.2}", budgeted)} }
                        }
                        div { class: "overview-card spent-card",
                            div { class: "card-header", "Total Spent" }
                            div { class: "card-amount", {format_args!("${:.2}", spent)} }
                        }
                        div { class: {format_args!("overview-card unallocated-card {}", unallocated_class)},
                            div { class: "card-header", "Unallocated" }
                            div { class: "card-amount", {format_args!("${:.2}", unallocated)} }
                        }
                    }

                    // Progress bar
                    div { class: "budget-progress-section",
                        div { class: "progress-label", "Budgeted vs Income" }
                        div { class: "progress-bar-container",
                            div { class: "progress-bar", style: {format!("width: {:.0}%;", progress_clamped)} }
                        }
                        div { class: "progress-text", {format_args!("{:.0}% of income is budgeted", progress)} }
                    }

                    // Best items section
                    {
                        if budget.item_summaries.is_empty() {
                            rsx! {
                                div { class: "no-issues-section",
                                    div { class: "no-issues-icon", "🎉" }
                                    div { class: "no-issues-text", "No items" }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "issues-section",
                                    h3 { class: "issues-title", "Items with money left to spend" }
                                    div { class: "issues-list",
                                        for item in budget.item_summaries.iter().take(5) {
                                            div { class: "issue-content",
                                                div { class: "issue-description", {item.name.clone()} }
                                                div { class: "issue-amount", {format_args!("Left: ${:.2}", item.left_to_spend)} }
                                                div { class: "issue-amount", {format_args!("Budgeted: ${:.2}", item.budgeted_amount)} }                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Issues section
                    {
                        if budget.issues.is_empty() {
                            rsx! {
                                div { class: "no-issues-section",
                                    div { class: "no-issues-icon", "🎉" }
                                    div { class: "no-issues-text", "No issues to show. Great job!" }
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "issues-section",
                                    h3 { class: "issues-title", "Issues to review" }
                                    div { class: "issues-list",
                                        for issue in budget.issues.iter() {
                                            div { key: "{issue.issue_type}", class: {format_args!("issue-item {}", match &issue.issue_type { BudgetIssueType::Overspent(_) => "overspent", BudgetIssueType::Unbalanced => "unbalanced", BudgetIssueType::TransactionNotConnected(_) => "unconnected", })},
                                                div { class: "issue-icon", {match &issue.issue_type { BudgetIssueType::Overspent(_) => "🔥", BudgetIssueType::Unbalanced => "⚠", BudgetIssueType::TransactionNotConnected(_) => "🔗", }} }
                                                div { class: "issue-content",
                                                    div { class: "issue-description", {issue.description.as_str()} }
                                                    div { class: "issue-description", BudgetPopover { max_amount: issue.amount } }
                                                    div { class: "issue-amount", {format_args!("Amount: ${:.2}", issue.amount)} }
                                                button {
                                                        class: "button",
                                                        "Budget"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Quick stats
                    div { class: "quick-stats",
                        div { class: "stat-item",
                            span { class: "stat-label", "Default budget" }
                            span { class: "stat-value {default_class}",
                                {default_text}
                            }
                        }
                        div { class: "stat-item",
                            span { class: "stat-label", "Balance" }
                            span { class: "stat-value {balance_value_class}",
                                {format_args!("${:.2}", income - spent)}
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
