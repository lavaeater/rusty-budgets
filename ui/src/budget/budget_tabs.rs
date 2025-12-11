use crate::budget::{BudgetingTypeCard, BudgetingTypeOverviewView};
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::view_models::*;
use api::models::*;
use dioxus::prelude::*;
use uuid::Uuid;
use api::view_models::BudgetItemViewModel;
use api::view_models::BudgetViewModel;
use api::view_models::BudgetingTypeOverview;
use crate::budget::budget_hero::BudgetState;

#[component]
pub fn BudgetTabs() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
            let overview_by_type = budget_signal()
                .overviews
                .iter()
                .enumerate()
                .map(|(index, ov)| {
                    (
                        index,
                        ov.budgeting_type,
                        ov.clone(),
                        budget_signal()
                            .items
                            .iter()
                            .filter(|i| i.budgeting_type == ov.budgeting_type)
                            .cloned()
                            .collect(),
                    )
                })
                .collect::<Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItemViewModel>)>>();

            rsx! {
                Tabs {
                    class: "dashboard-cards",
                    default_value: overview_by_type.first().unwrap().1.to_string(),
                    horizontal: true,
                    TabList { class: "dashboard-cards",
                        for (index , budgeting_type , overview , _) in &overview_by_type {
                            TabTrigger {
                                value: budgeting_type.to_string(),
                                index: *index,
                                BudgetingTypeOverviewView {
                                    budgeting_type: *budgeting_type,
                                    overview: *overview,
                                }
                            }
                        }
                    }
                    for (index , budgeting_type , _ , items) in overview_by_type {
                        TabContent { index, value: budgeting_type.to_string(),
                            BudgetingTypeCard { budgeting_type, items }
                        }
                    }
                }
            }
        }
