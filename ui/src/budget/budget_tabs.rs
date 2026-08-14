use crate::budget::budget_hero::BudgetState;
use crate::budget::{BudgetingTypeCard, BudgetingTypeOverviewView};
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::BudgetingType;
use api::view_models::BudgetItemViewModel;
use api::view_models::BudgetViewModel;
use api::view_models::BudgetingTypeOverview;
use api::view_models::*;
use dioxus::prelude::*;
use uuid::Uuid;

/// Tabs across the four `BudgetingType`s (Inkomst/Utgift/Sparande/Överföring).
///
/// This is a filter *within* the budget table — not to be confused with the
/// workspace-level [`crate::budget::BudgetTab`], which switches task view.
#[component]
pub fn BudgetingTypeTabs() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let overview_by_type = budget_signal()
        .overviews
        .iter()
        .enumerate()
        .map(|(index, ov)| {
            (
                index,
                ov.budgeting_type,
                *ov,
                budget_signal()
                    .items
                    .iter()
                    .filter(|i| i.budgeting_type == ov.budgeting_type)
                    .cloned()
                    .collect(),
            )
        })
        .collect::<Vec<(
            usize,
            BudgetingType,
            BudgetingTypeOverview,
            Vec<BudgetItemViewModel>,
        )>>();

    // A loaded budget always projects all four `BudgetingType` overviews, but a
    // default/empty view model has none — and this is now reachable by clicking
    // the Budget tab, so it must not panic.
    let Some((_, first_type, _, _)) = overview_by_type.first() else {
        return rsx! {
            p { class: "report-empty", "Ingen budgetdata för den här perioden ännu." }
        };
    };

    rsx! {
        Tabs {
            class: "dashboard-cards",
            default_value: first_type.to_string(),
            horizontal: true,
            TabList { class: "dashboard-cards",
                for (index, budgeting_type, overview, _) in &overview_by_type {
                    TabTrigger { value: budgeting_type.to_string(), index: *index,
                        BudgetingTypeOverviewView {
                            budgeting_type: *budgeting_type,
                            overview: *overview,
                        }
                    }
                }
            }
            for (index, budgeting_type, _, _) in overview_by_type {
                TabContent { index, value: budgeting_type.to_string(),
                    BudgetingTypeCard { budgeting_type }
                }
            }
        }
    }
}
