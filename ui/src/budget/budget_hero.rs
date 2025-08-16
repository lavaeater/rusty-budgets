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
                class: "budget-hero-container",

                // Header Section
                div {
                    class: "budget-header",
                    h1 { class: "budget-title", "{budget.name}" }
                    div {
                        class: if budget.is_balanced { "balance-status balanced" } else { "balance-status unbalanced" },
                        span { class: "status-icon", if budget.is_balanced { "✓" } else { "⚠" } }
                        span { if budget.is_balanced { "Balanced" } else { "Needs Attention" } }
                    }
                }

                // Financial Overview Cards
                div {
                    class: "financial-overview",

                    div { class: "overview-card income-card",
                        div { class: "card-header", "Total Income" }
                        div { class: "card-amount", "{budget.total_income:,.0} SEK" }
                    }

                    div { class: "overview-card budgeted-card",
                        div { class: "card-header", "Total Budgeted" }
                        div { class: "card-amount", "{budget.total_budgeted:,.0} SEK" }
                    }

                    div { class: "overview-card spent-card",
                        div { class: "card-header", "Total Spent" }
                        div { class: "card-amount", "{budget.total_spent:,.0} SEK" }
                    }

                    div {
                        class: if budget.unallocated_income > 0.0 { "overview-card unallocated-card positive" } else { "overview-card unallocated-card" },
                        div { class: "card-header", "Unallocated" }
                        div { class: "card-amount", "{budget.unallocated_income:,.0} SEK" }
                    }
                }

                // Budget Progress Section
                if budget.total_income > 0.0 {
                    div {
                        class: "budget-progress-section",
                        div { class: "progress-label", "Budget Utilization" }
                        div {
                            class: "progress-bar-container",
                            div {
                                class: "progress-bar",
                                style: "width: {((budget.total_budgeted / budget.total_income) * 100.0).min(100.0)}%"
                            }
                        }
                        div { class: "progress-text",
                            "{(budget.total_budgeted / budget.total_income * 100.0):.1}% of income allocated"
                        }
                    }
                }

                // Issues Section
                if !budget.issues.is_empty() {
                    div {
                        class: "issues-section",
                        h3 { class: "issues-title", "Issues Requiring Attention ({budget.issues.len()})" }
                        div {
                            class: "issues-list",
                            for issue in &budget.issues {
                                div {
                                    class: match &issue.issue_type {
                                        BudgetIssueType::Overspent(_) => "issue-item overspent",
                                        BudgetIssueType::Unbalanced => "issue-item unbalanced",
                                        BudgetIssueType::TransactionNotConnected(_) => "issue-item unconnected",
                                    },
                                    div {
                                        class: "issue-icon",
                                        match &issue.issue_type {
                                            BudgetIssueType::Overspent(_) => "💸",
                                            BudgetIssueType::Unbalanced => "⚖️",
                                            BudgetIssueType::TransactionNotConnected(_) => "🔗",
                                        }
                                    }
                                    div {
                                        class: "issue-content",
                                        div { class: "issue-description", "{issue.description}" }
                                        div { class: "issue-amount", "{issue.amount:,.0} SEK" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div {
                        class: "no-issues-section",
                        div { class: "no-issues-icon", "🎉" }
                        div { class: "no-issues-text", "Great! No issues found with your budget." }
                    }
                }

                // Quick Stats
                div {
                    class: "quick-stats",
                    div { class: "stat-item",
                        span { class: "stat-label", "Spending Rate" }
                        span {
                            class: if (budget.total_budgeted > 0.0 &&
                                (budget.total_spent / budget.total_budgeted * 100.0) > 90.0) { "stat-value warning" }
                                else { "stat-value good" },
                            if budget.total_budgeted > 0.0 {
                              "Birji"  // format!("{:.1}%", budget.total_spent / budget.total_budgeted * 100.0)
                            } else {
                                "N/A"
                            }
                        }
                    }
                    div { class: "stat-item",
                        span { class: "stat-label", "Remaining Budget" }
                        span {
                            class: if budget.total_budgeted - budget.total_spent > 0.0 { "stat-value positive" } else { "stat-value warning" },
                            "{(budget.total_budgeted - budget.total_spent):,.0} SEK"
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
