use crate::budget::BudgetingTypeCard;
use crate::components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget::BudgetingTypeOverviewView;

#[component]
pub fn BudgetTabs(
) -> Element {
    let budget_signal = use_context::<Signal<Option<Budget>>>();
     match budget_signal() {
        Some(budget) => {
            let items_by_type = budget.items_by_type();
            rsx! {
                Tabs {
                    default_value: items_by_type.first().unwrap().1.to_string(),
                    horizontal: true,
                    TabList {
                        for (index , budgeting_type , overview , _) in &items_by_type {
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
                    for (index , budgeting_type , _ , items) in items_by_type {
                        TabContent { index, value: budgeting_type.to_string(),
                            BudgetingTypeCard { budgeting_type, items }
                        }
                    }
                }
            }
        }
        None => {
            rsx! {
                h1 { "Ingen budget - än" }
            }
        }
    }
}
