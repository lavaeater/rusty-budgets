use crate::budget::BudgetingTypeCard;
use crate::budget_components::{TabContent, TabList, TabTrigger, Tabs};
use api::models::{Budget, BudgetItem, BudgetingType, BudgetingTypeOverview};
use dioxus::prelude::*;
use uuid::Uuid;
use crate::budget::BudgetingTypeOverviewView;

#[component]
pub fn BudgetingTypeTabs(
    budget_id: Uuid,
    items_by_type: Vec<(usize, BudgetingType, BudgetingTypeOverview, Vec<BudgetItem>)>,
) -> Element {
    rsx! {
        Tabs {
            default_value: items_by_type.first().unwrap().1.to_string(),
            horizontal: true,
            TabList {
                for (index , budgeting_type , overview , _) in &items_by_type {
                    TabTrigger { value: budgeting_type.to_string(), index: *index,
                        BudgetingTypeOverviewView {
                            budget_id,
                            budgeting_type: *budgeting_type,
                            overview: *overview,
                        }
                    }
                }
            }
            for (index , budgeting_type , _ , items) in items_by_type {
                TabContent { index, value: budgeting_type.to_string(),
                    BudgetingTypeCard { budget_id, budgeting_type, items }
                }
            }
        }
    }
}
