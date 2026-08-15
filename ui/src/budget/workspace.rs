use crate::budget::budget_hero::{BudgetState, HERO_CSS};
use crate::budget::tabs::{
    BudgetPlanTab, OverviewTab, ReportsTab, SettingsTab, TodoTab, TransactionsTab,
};
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{BudgetingType, MonthBeginsOn, PeriodId};
use api::view_models::BudgetViewModel;
use chrono::Utc;
use dioxus::prelude::*;
use std::fmt;
use std::str::FromStr;
use strum::{EnumIter, IntoEnumIterator};
use uuid::Uuid;

const WORKSPACE_CSS: Asset = asset!("assets/styling/workspace.css");

/// The task-oriented views available within a single budget period.
///
/// Each variant is a distinct place to stand rather than another section on one
/// long page. Ordering here is the display order of the tab bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, EnumIter)]
pub enum BudgetTab {
    #[default]
    Overview,
    Plan,
    Transactions,
    Todo,
    Reports,
    Settings,
}

impl BudgetTab {
    /// URL-safe identifier, also used as the `Tabs` primitive's value.
    pub fn slug(self) -> &'static str {
        match self {
            BudgetTab::Overview => "oversikt",
            BudgetTab::Plan => "budget",
            BudgetTab::Transactions => "transaktioner",
            BudgetTab::Todo => "att-gora",
            BudgetTab::Reports => "rapporter",
            BudgetTab::Settings => "installningar",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            BudgetTab::Overview => "Översikt",
            BudgetTab::Plan => "Budget",
            BudgetTab::Transactions => "Transaktioner",
            BudgetTab::Todo => "Att göra",
            BudgetTab::Reports => "Rapporter",
            BudgetTab::Settings => "Inställningar",
        }
    }

    /// Count of outstanding work shown as a badge on the tab, if any.
    fn badge_count(self, budget: &BudgetViewModel) -> usize {
        match self {
            BudgetTab::Todo => {
                budget.untagged_transaction_count
                    + budget.potential_transfer_count
                    + budget.to_connect.len()
            }
            _ => 0,
        }
    }
}

impl fmt::Display for BudgetTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.slug())
    }
}

impl FromStr for BudgetTab {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        BudgetTab::iter().find(|t| t.slug() == s).ok_or(())
    }
}

/// The period workspace: a stable header plus one task-oriented tab at a time.
///
/// Replaces the previous single-page `BudgetOverview`, which stacked twelve
/// conditional sections in one scroll.
#[component]
pub fn BudgetWorkspace(
    mut budget_id: Signal<Uuid>,
    period_id: Signal<PeriodId>,
    tab: Signal<BudgetTab>,
) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    budget_id.set(budget.id);

    rsx! {
        // Both sheets are required: `budget-hero.css` still owns the container,
        // header, RTA badge, dashboard/overview cards and progress bars that the
        // tab panels reuse; `workspace.css` adds only the tab-bar-era rules.
        document::Link { rel: "stylesheet", href: HERO_CSS }
        document::Link { rel: "stylesheet", href: WORKSPACE_CSS }
        div { class: "budget-hero-a-container",
            BudgetWorkspaceHeader { period_id }

            Tabs {
                class: "workspace-tabs",
                horizontal: true,
                value: tab().slug().to_string(),
                on_value_change: move |value: String| {
                    if let Ok(next) = BudgetTab::from_str(&value) {
                        tab.set(next);
                    }
                },
                TabList { class: "workspace-tab-list",
                    for (index , candidate) in BudgetTab::iter().enumerate() {
                        TabTrigger {
                            value: candidate.slug().to_string(),
                            index,
                            span { class: "workspace-tab-label", {candidate.label()} }
                            {
                                let count = candidate.badge_count(&budget);
                                (count > 0)
                                    .then(|| rsx! {
                                        span { class: "workspace-tab-badge", {count.to_string()} }
                                    })
                            }
                        }
                    }
                }
                // Panels are generated from the same iteration order as the
                // triggers above, so the two can never disagree on an index.
                for (index , candidate) in BudgetTab::iter().enumerate() {
                    TabContent { index, value: candidate.slug().to_string(),
                        match candidate {
                            BudgetTab::Overview => rsx! {
                                OverviewTab { tab }
                            },
                            BudgetTab::Plan => rsx! {
                                BudgetPlanTab {}
                            },
                            BudgetTab::Transactions => rsx! {
                                TransactionsTab {}
                            },
                            BudgetTab::Todo => rsx! {
                                TodoTab {}
                            },
                            BudgetTab::Reports => rsx! {
                                ReportsTab {}
                            },
                            BudgetTab::Settings => rsx! {
                                SettingsTab {}
                            },
                        }
                    }
                }
            }
        }
    }
}

/// Budget name, period navigation and the "Att fördela" figure.
///
/// Stays fixed above the tab bar: these are the facts you need no matter which
/// task you are doing.
#[component]
fn BudgetWorkspaceHeader(mut period_id: Signal<PeriodId>) -> Element {
    let budget = use_context::<BudgetState>().0();
    let period_id_now = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());

    let ready_to_assign = budget
        .overviews
        .iter()
        .find(|ov| ov.budgeting_type == BudgetingType::Income)
        .map(|ov| ov.remaining_budget);

    rsx! {
        div { class: "budget-header-a",
            div { class: "header-title",
                h1 { {budget.name.clone()} }
                div { class: "period-nav",
                    button {
                        class: "period-nav-btn",
                        onclick: move |_| { period_id.set(period_id().month_before()); },
                        "‹"
                    }
                    span { class: "period-nav-label", {period_id().to_string()} }
                    button {
                        class: "period-nav-btn",
                        onclick: move |_| { period_id.set(period_id().month_after()); },
                        "›"
                    }
                }
            }
            div { class: "header-actions",
                if let Some(rta) = ready_to_assign {
                    {
                        let cents = rta.amount_in_cents();
                        let rta_class = match cents.cmp(&0) {
                            std::cmp::Ordering::Less => "rta-badge rta-over",
                            std::cmp::Ordering::Equal => "rta-badge rta-balanced",
                            std::cmp::Ordering::Greater => "rta-badge rta-under",
                        };
                        let label = if cents < 0 { "Överbudgeterat" } else { "Att fördela" };
                        rsx! {
                            div { class: rta_class,
                                span { class: "rta-label", {label} }
                                span { class: "rta-amount", {rta.to_string()} }
                            }
                        }
                    }
                }
            }
        }

        if period_id() != period_id_now {
            div { class: "past-period-banner",
                span { "Du ser en tidigare period — " {period_id().to_string()} }
                button {
                    class: "past-period-go-now",
                    onclick: move |_| period_id.set(period_id_now),
                    "Gå till nuvarande månad →"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_round_trip() {
        for tab in BudgetTab::iter() {
            assert_eq!(BudgetTab::from_str(tab.slug()), Ok(tab));
        }
    }

    #[test]
    fn slugs_are_unique_and_url_safe() {
        let slugs: Vec<&str> = BudgetTab::iter().map(BudgetTab::slug).collect();
        let unique: std::collections::HashSet<&&str> = slugs.iter().collect();
        assert_eq!(slugs.len(), unique.len());
        for slug in slugs {
            assert!(
                slug.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '-'),
                "slug {slug} is not url-safe"
            );
        }
    }

    #[test]
    fn unknown_slug_is_rejected() {
        assert_eq!(BudgetTab::from_str("nope"), Err(()));
    }

    #[test]
    fn default_tab_is_overview() {
        assert_eq!(BudgetTab::default(), BudgetTab::Overview);
    }
}
