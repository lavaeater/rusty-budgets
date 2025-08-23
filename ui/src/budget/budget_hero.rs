use api::models::*;
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget_popover::BudgetPopover;

const BUDGET_CSS: Asset = asset!("/assets/styling/budget.css");

pub static CURRENT_BUDGET_ID: GlobalSignal<Uuid> = Signal::global(|| Uuid::default());

// VARIANT 1: Original Design (Card-based layout)
#[component]
pub fn BudgetHeroOne() -> Element {
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

// VARIANT 2: Minimalist Dashboard
#[component]
pub fn BudgetHeroTwo() -> Element {
    let budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetSummary>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    match budget_signal() {
        Some(budget) => {
            let income = budget.total_income;
            let spent = budget.total_spent;
            let remaining = income - spent;
            let progress = if income > 0.0 { (spent / income) * 100.0 } else { 0.0 };
            
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-minimal",
                    // Clean header with large numbers
                    div { class: "minimal-header",
                        h1 { class: "minimal-title", {budget.name} }
                        div { class: "balance-indicator",
                            if budget.is_balanced { "✓ Balanced" } else { "⚠ Needs Attention" }
                        }
                    }
                    
                    // Large financial display
                    div { class: "financial-display",
                        div { class: "main-balance",
                            div { class: "balance-label", "Available" }
                            div { class: "balance-amount", {format_args!("${:.0}", remaining)} }
                        }
                        div { class: "spending-bar",
                            div { class: "spent-portion", style: {format!("width: {:.0}%", progress.clamp(0.0, 100.0))} }
                            div { class: "spending-text", {format_args!("${:.0} of ${:.0} spent", spent, income)} }
                        }
                    }
                    
                    // Compact issues
                    if !budget.issues.is_empty() {
                        div { class: "minimal-issues",
                            for issue in budget.issues.iter().take(3) {
                                div { class: "minimal-issue",
                                    span { class: "issue-emoji", 
                                        {match &issue.issue_type {
                                            BudgetIssueType::Overspent(_) => "🔥",
                                            BudgetIssueType::Unbalanced => "⚠",
                                            BudgetIssueType::TransactionNotConnected(_) => "🔗",
                                        }}
                                    }
                                    span { {issue.description.as_str()} }
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "minimal-loading", "Loading budget..." }
            }
        }
    }
}

// VARIANT 3: Data-Heavy Analytics View
#[component]
pub fn BudgetHeroThree() -> Element {
    let budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetSummary>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    match budget_signal() {
        Some(budget) => {
            let income = budget.total_income;
            let budgeted = budget.total_budgeted;
            let spent = budget.total_spent;
            let unallocated = budget.unallocated_income;
            let efficiency = if budgeted > 0.0 { (spent / budgeted) * 100.0 } else { 0.0 };
            
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-analytics",
                    // Analytics header
                    div { class: "analytics-header",
                        div { class: "budget-name-analytics", {budget.name} }
                        div { class: "status-badges",
                            div { class: if budget.is_balanced { "badge balanced" } else { "badge unbalanced" },
                                {if budget.is_balanced { "Balanced" } else { "Unbalanced" }}
                            }
                            div { class: if budget.default_budget { "badge default" } else { "badge non-default" },
                                {if budget.default_budget { "Default" } else { "Custom" }}
                            }
                        }
                    }
                    
                    // Metrics grid
                    div { class: "metrics-grid",
                        div { class: "metric-box primary",
                            div { class: "metric-value", {format_args!("${:.0}", income)} }
                            div { class: "metric-label", "Total Income" }
                            div { class: "metric-change", "100%" }
                        }
                        div { class: "metric-box secondary",
                            div { class: "metric-value", {format_args!("${:.0}", budgeted)} }
                            div { class: "metric-label", "Budgeted" }
                            div { class: "metric-change", {format_args!("{:.0}%", if income > 0.0 { (budgeted/income)*100.0 } else { 0.0 })} }
                        }
                        div { class: "metric-box tertiary",
                            div { class: "metric-value", {format_args!("${:.0}", spent)} }
                            div { class: "metric-label", "Spent" }
                            div { class: "metric-change", {format_args!("{:.0}%", efficiency)} }
                        }
                        div { class: "metric-box quaternary",
                            div { class: "metric-value", {format_args!("${:.0}", unallocated)} }
                            div { class: "metric-label", "Unallocated" }
                            div { class: if unallocated > 0.0 { "metric-change positive" } else { "metric-change" }, 
                                {format_args!("{:.0}%", if income > 0.0 { (unallocated/income)*100.0 } else { 0.0 })}
                            }
                        }
                    }
                    
                    // Detailed breakdown
                    div { class: "breakdown-section",
                        h3 { "Budget Breakdown" }
                        div { class: "breakdown-bars",
                            div { class: "breakdown-item",
                                div { class: "breakdown-label", "Budgeted" }
                                div { class: "breakdown-bar",
                                    div { class: "bar-fill budgeted", style: {format!("width: {:.0}%", if income > 0.0 { (budgeted/income)*100.0 } else { 0.0 })} }
                                }
                                div { class: "breakdown-value", {format_args!("${:.0}", budgeted)} }
                            }
                            div { class: "breakdown-item",
                                div { class: "breakdown-label", "Spent" }
                                div { class: "breakdown-bar",
                                    div { class: "bar-fill spent", style: {format!("width: {:.0}%", if income > 0.0 { (spent/income)*100.0 } else { 0.0 })} }
                                }
                                div { class: "breakdown-value", {format_args!("${:.0}", spent)} }
                            }
                        }
                    }
                    
                    // Issues table
                    if !budget.issues.is_empty() {
                        div { class: "issues-table",
                            h3 { "Action Items" }
                            div { class: "table-header",
                                div { "Type" }
                                div { "Description" }
                                div { "Amount" }
                                div { "Action" }
                            }
                            for issue in budget.issues.iter() {
                                div { class: "table-row",
                                    div { class: "issue-type",
                                        {match &issue.issue_type {
                                            BudgetIssueType::Overspent(_) => "Overspent",
                                            BudgetIssueType::Unbalanced => "Unbalanced",
                                            BudgetIssueType::TransactionNotConnected(_) => "Unconnected",
                                        }}
                                    }
                                    div { class: "issue-desc", {issue.description.as_str()} }
                                    div { class: "issue-amt", {format_args!("${:.2}", issue.amount)} }
                                    div { class: "issue-action",
                                        button { class: "action-btn", "Fix" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "analytics-loading", "Loading analytics..." }
            }
        }
    }
}

// VARIANT 4: Mobile-First Compact Design
#[component]
pub fn BudgetHeroFour() -> Element {
    let budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetSummary>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    match budget_signal() {
        Some(budget) => {
            let income = budget.total_income;
            let spent = budget.total_spent;
            let remaining = income - spent;
            
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-mobile",
                    // Compact header
                    div { class: "mobile-header",
                        div { class: "budget-title-mobile", {budget.name} }
                        div { class: "status-dot", class: if budget.is_balanced { "balanced" } else { "unbalanced" } }
                    }
                    
                    // Main balance card
                    div { class: "balance-card",
                        div { class: "balance-main",
                            div { class: "balance-amount-large", {format_args!("${:.0}", remaining)} }
                            div { class: "balance-label-small", "Available to spend" }
                        }
                        div { class: "balance-breakdown",
                            div { class: "breakdown-item-small",
                                span { class: "amount", {format_args!("${:.0}", income)} }
                                span { class: "label", "Income" }
                            }
                            div { class: "breakdown-item-small",
                                span { class: "amount", {format_args!("${:.0}", spent)} }
                                span { class: "label", "Spent" }
                            }
                        }
                    }
                    
                    // Progress indicator
                    div { class: "progress-mobile",
                        div { class: "progress-circle",
                            div { class: "progress-text", {format_args!("{:.0}%", if income > 0.0 { (spent/income)*100.0 } else { 0.0 })} }
                        }
                        div { class: "progress-label", "Budget used" }
                    }
                    
                    // Quick actions
                    div { class: "quick-actions",
                        button { class: "action-button primary", "Add Expense" }
                        button { class: "action-button secondary", "View Details" }
                    }
                    
                    // Compact issues
                    if !budget.issues.is_empty() {
                        div { class: "mobile-alerts",
                            div { class: "alert-count", {format_args!("{} items need attention", budget.issues.len())} }
                            for issue in budget.issues.iter().take(2) {
                                div { class: "alert-item",
                                    div { class: "alert-icon",
                                        {match &issue.issue_type {
                                            BudgetIssueType::Overspent(_) => "🔥",
                                            BudgetIssueType::Unbalanced => "⚠",
                                            BudgetIssueType::TransactionNotConnected(_) => "🔗",
                                        }}
                                    }
                                    div { class: "alert-text", {issue.description.as_str()} }
                                }
                            }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "mobile-loading", "Loading..." }
            }
        }
    }
}

// VARIANT 5: Gamified Progress Design
#[component]
pub fn BudgetHeroFive() -> Element {
    let budget_resource = use_server_future(api::get_default_budget_overview)?;
    let mut budget_signal = use_signal(|| None::<BudgetSummary>);

    use_effect(move || {
        if let Some(Ok(budget)) = budget_resource.read().as_ref() {
            *CURRENT_BUDGET_ID.write() = budget.id;
            budget_signal.set(Some(budget.clone()));
        }
    });

    match budget_signal() {
        Some(budget) => {
            let income = budget.total_income;
            let spent = budget.total_spent;
            let budgeted = budget.total_budgeted;
            let efficiency_score = if budgeted > 0.0 && spent <= budgeted { 
                ((budgeted - spent) / budgeted * 100.0).clamp(0.0, 100.0) 
            } else { 0.0 };
            let level = (efficiency_score / 20.0).floor() as i32 + 1;
            let streak = if budget.is_balanced { 7 } else { 0 }; // Mock streak data
            
            rsx! {
                document::Link { rel: "stylesheet", href: BUDGET_CSS }
                div { class: "budget-hero-gamified",
                    // Gamified header
                    div { class: "game-header",
                        div { class: "budget-title-game", {budget.name} }
                        div { class: "level-badge", "Level {level}" }
                        div { class: "streak-counter", "🔥 {streak} day streak" }
                    }
                    
                    // Score display
                    div { class: "score-section",
                        div { class: "score-circle",
                            div { class: "score-value", {format_args!("{:.0}", efficiency_score)} }
                            div { class: "score-label", "Efficiency Score" }
                        }
                        div { class: "score-breakdown",
                            div { class: "score-item",
                                span { class: "score-icon", "💰" }
                                span { class: "score-text", {format_args!("${:.0} saved", budgeted - spent)} }
                            }
                            div { class: "score-item",
                                span { class: "score-icon", if budget.is_balanced { "✅" } else { "❌" } }
                                span { class: "score-text", if budget.is_balanced { "Budget balanced" } else { "Needs balancing" } }
                            }
                        }
                    }
                    
                    // Progress bars with achievements
                    div { class: "progress-achievements",
                        div { class: "achievement-bar",
                            div { class: "achievement-label", "Monthly Goal" }
                            div { class: "progress-bar-game",
                                div { class: "progress-fill", style: {format!("width: {:.0}%", (spent/income*100.0).clamp(0.0, 100.0))} }
                            }
                            div { class: "achievement-text", {format_args!("{:.0}% of budget used", spent/income*100.0)} }
                        }
                        
                        div { class: "achievement-bar",
                            div { class: "achievement-label", "Savings Target" }
                            div { class: "progress-bar-game savings",
                                div { class: "progress-fill", style: {format!("width: {:.0}%", efficiency_score)} }
                            }
                            div { class: "achievement-text", {format_args!("{:.0}% efficiency", efficiency_score)} }
                        }
                    }
                    
                    // Challenges/Issues as quests
                    div { class: "quest-section",
                        h3 { class: "quest-title", "🎯 Active Quests" }
                        if budget.issues.is_empty() {
                            div { class: "no-quests",
                                div { class: "quest-complete-icon", "🏆" }
                                div { class: "quest-complete-text", "All quests completed! You're a budget master!" }
                            }
                        } else {
                            div { class: "quest-list",
                                for (i, issue) in budget.issues.iter().enumerate() {
                                    div { class: "quest-item",
                                        div { class: "quest-icon",
                                            {match &issue.issue_type {
                                                BudgetIssueType::Overspent(_) => "⚔️",
                                                BudgetIssueType::Unbalanced => "⚖️",
                                                BudgetIssueType::TransactionNotConnected(_) => "🔗",
                                            }}
                                        }
                                        div { class: "quest-content",
                                            div { class: "quest-name", 
                                                {match &issue.issue_type {
                                                    BudgetIssueType::Overspent(_) => "Defeat the Overspender",
                                                    BudgetIssueType::Unbalanced => "Restore Balance",
                                                    BudgetIssueType::TransactionNotConnected(_) => "Connect the Missing Link",
                                                }}
                                            }
                                            div { class: "quest-description", {issue.description.as_str()} }
                                            div { class: "quest-reward", {format_args!("Reward: ${:.0} XP", issue.amount)} }
                                        }
                                        button { class: "quest-button", "Accept Quest" }
                                    }
                                }
                            }
                        }
                    }
                    
                    // Achievement badges
                    div { class: "badges-section",
                        div { class: "badge-item", class: if budget.is_balanced { "earned" } else { "locked" },
                            div { class: "badge-icon", "🎯" }
                            div { class: "badge-name", "Balanced Budget" }
                        }
                        div { class: "badge-item", class: if efficiency_score > 80.0 { "earned" } else { "locked" },
                            div { class: "badge-icon", "💎" }
                            div { class: "badge-name", "Efficiency Master" }
                        }
                        div { class: "badge-item", class: if budget.issues.is_empty() { "earned" } else { "locked" },
                            div { class: "badge-icon", "🏆" }
                            div { class: "badge-name", "Problem Solver" }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                div { class: "game-loading", "Loading your budget adventure..." }
            }
        }
    }
}
