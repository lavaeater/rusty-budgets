use crate::Route;
use dioxus::prelude::*;
use ui::budget::{BudgetHero, BudgetLocation};

fn route_for(location: BudgetLocation) -> Route {
    Route::Budget {
        period: location.period_slug(),
        tab: location.tab_slug().to_string(),
    }
}

/// `/budget/:period/:tab` — the workspace, with the URL as the source of truth
/// for which period and task view is open.
#[component]
pub fn Budget(period: String, tab: String) -> Element {
    let navigator = use_navigator();

    let Some(location) = BudgetLocation::from_slugs(&period, &tab) else {
        return rsx! {
            div { class: "budget-hero-a-container",
                h4 { "Okänd adress" }
                p { "Perioden eller vyn i adressen kunde inte tolkas." }
                Link { to: route_for(BudgetLocation::current()), "Gå till nuvarande månad →" }
            }
        };
    };

    rsx! {
        BudgetHero {
            location,
            // `push`, not `replace`: switching tab or period is a place the user
            // expects the back button to return them from.
            on_navigate: move |next: BudgetLocation| {
                navigator.push(route_for(next));
            },
        }
    }
}
