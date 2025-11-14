use api::models::{Budget, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::view_models::BudgetingTypeOverview;

#[component]
pub fn BudgetingTypeOverviewView(
    budgeting_type: BudgetingType,
    overview: BudgetingTypeOverview,
) -> Element {
            match budgeting_type {
                BudgetingType::Income => {
                    rsx! {
                        div {
                            h2 { {budgeting_type.to_string()} }
                            div {
                                p {
                                    "Budgeterat "
                                    {overview.budgeted_amount.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Återstår: "
                                    {overview.remaining_budget.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Inkommet: "
                                    {overview.actual_amount.to_string()}
                                }
                            }
                        }
                    }
                }
                BudgetingType::Expense => {
                    rsx! {
                        div {
                            h2 { {budgeting_type.to_string()} }
                            div {
                                p {
                                    "Budgeterat "
                                    {overview.budgeted_amount.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Återstår: "
                                    {overview.remaining_budget.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Spenderat: "
                                    {overview.actual_amount.to_string()}
                                }
                            }
                        }
                    }
                }
                BudgetingType::Savings => {
                    rsx! {
                        div {
                            h2 { {budgeting_type.to_string()} }
                            div {
                                p {
                                    "Budgeterat "
                                    {overview.budgeted_amount.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Återstår: "
                                    {overview.remaining_budget.to_string()}
                                }
                            }
                            div {
                                p {
                                    "Inkommet: "
                                    {overview.actual_amount.to_string()}
                                }
                            }
                        }
                    }
                }
            }            
        }
