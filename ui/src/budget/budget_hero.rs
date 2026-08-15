use crate::budget::workspace::{BudgetTab, BudgetWorkspace};
use api::models::{MonthBeginsOn, PeriodId};
use api::view_models::BudgetViewModel;
use api::get_budget;
use chrono::Utc;
use dioxus::prelude::*;
use dioxus_primitives::label::Label;
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

/// Where the user is within the budget: which period, and which task view.
///
/// Platforms that have a router (web) map this to and from the URL; platforms
/// that don't (desktop, mobile) simply let `BudgetHero` own it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetLocation {
    pub period_id: PeriodId,
    pub tab: BudgetTab,
}

impl BudgetLocation {
    /// The period containing today, on the default tab — where a session with
    /// no explicit destination should land.
    pub fn current() -> Self {
        Self {
            period_id: PeriodId::from_date(Utc::now(), MonthBeginsOn::default()),
            tab: BudgetTab::default(),
        }
    }

    /// Parses the two URL segments produced by [`Self::period_slug`] and
    /// [`BudgetTab::slug`]. Returns `None` if either segment is unrecognised.
    pub fn from_slugs(period: &str, tab: &str) -> Option<Self> {
        Some(Self {
            period_id: period.parse().ok()?,
            tab: tab.parse().ok()?,
        })
    }

    pub fn period_slug(&self) -> String {
        self.period_id.to_string()
    }

    pub fn tab_slug(&self) -> &'static str {
        self.tab.slug()
    }
}

/// Shared with [`crate::budget::workspace`] — it carries the container, header,
/// card and progress-bar rules the workspace still relies on.
pub(crate) const HERO_CSS: Asset = asset!("assets/styling/budget-hero.css");

/// Loads the budget and hands off to [`BudgetWorkspace`].
///
/// `location` and `on_navigate` are optional: pass both to let a router drive
/// (and observe) the current period and tab; omit them and `BudgetHero` keeps
/// that state internally.
#[component]
pub fn BudgetHero(
    location: Option<BudgetLocation>,
    on_navigate: Option<EventHandler<BudgetLocation>>,
) -> Element {
    let mut budget_loading_state = use_signal(|| BudgetLoadingState::Loading);
    let budget_id = use_signal(Uuid::default);
    let mut period_id = use_signal(|| {
        location.map_or_else(
            || PeriodId::from_date(Utc::now(), MonthBeginsOn::default()),
            |l| l.period_id,
        )
    });
    let mut tab = use_signal(|| location.map(|l| l.tab).unwrap_or_default());

    // The last location we handed to the router. Guards both directions so a
    // URL echo doesn't bounce back as a fresh navigation.
    let mut published = use_signal(|| location);

    // Inbound: the router changed the URL (back/forward, or a deep link).
    // `use_reactive!` is required because `location` is a plain prop — a bare
    // `use_effect` would capture the first render's value and never see a change.
    use_effect(use_reactive!(|location| {
        if let Some(location) = location {
            published.set(Some(location));
            if *period_id.peek() != location.period_id {
                period_id.set(location.period_id);
            }
            if *tab.peek() != location.tab {
                tab.set(location.tab);
            }
        }
    }));

    // Outbound: the user navigated inside the workspace.
    use_effect(move || {
        let current = BudgetLocation {
            period_id: period_id(),
            tab: tab(),
        };
        if *published.peek() != Some(current) {
            published.set(Some(current));
            if let Some(on_navigate) = on_navigate {
                on_navigate.call(current);
            }
        }
    });

    let budget_resource = use_server_future(move || get_budget(None, period_id()))?;

    let mut budget_name = use_signal(String::new);
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
                BudgetOverview { budget_id, period_id, tab }
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
pub fn BudgetOverview(
    budget_id: Signal<Uuid>,
    period_id: Signal<PeriodId>,
    tab: Signal<BudgetTab>,
) -> Element {
    rsx! {
        BudgetWorkspace { budget_id, period_id, tab }
    }
}
