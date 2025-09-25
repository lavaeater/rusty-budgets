use std::collections::HashMap;
use dioxus::prelude::*;
use uuid::Uuid;
use api::models::{BudgetItem, BudgetingType};
use strum::IntoEnumIterator;
use crate::budget_components::{Tabs, TabList, TabTrigger, TabContent};
use crate::budget::BudgetingTypeCard;

#[component]
pub fn BudgetingTypeTabs(budget_id: Uuid, items_by_type: Vec<(usize,BudgetingType, Vec<BudgetItem>)>) -> Element {
    rsx! {
        Tabs {
            default_value: items_by_type.first().unwrap().1.to_string(),
            horizontal: true,
            max_width: "16rem",
            TabList {
                for bt in BudgetingType::iter() {
                    TabTrigger { value: bt.to_string(), index: 0usize, "{bt}" }
                }
            }
            for (index , budgeting_type , items) in items_by_type {
                TabContent { index, value: budgeting_type.to_string(),
                    BudgetingTypeCard { budget_id, budgeting_type, items }
                }
            }
        }
    }
}