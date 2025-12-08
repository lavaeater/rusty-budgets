use api::models::{Budget, BudgetingType};
use dioxus::prelude::*;
use uuid::Uuid;
use api::view_models::BudgetingTypeOverview;

#[component]
pub fn BudgetingTypeOverviewView(
    budgeting_type: BudgetingType,
    overview: BudgetingTypeOverview,
) -> Element {
            rsx! {
                div { class: format!("overview-card {}", if !overview.is_ok { "over-budget" } else { "" }),
                                h3 { 
                                    class: if !overview.is_ok { "warning" } else { "" },
                                    {budgeting_type.to_string()}
                                }
                                div { class: "card-stats",
                                    div { class: "stat",
                                        span { class: "stat-label", "Budgeterat" }
                                        span { class: "stat-value",
                                            {overview.budgeted_amount.to_string()}
                                        }
                                    }
                                    div { class: "stat",
                                        span { class: "stat-label", "Faktiskt" }
                                        span { class: "stat-value",
                                            {overview.actual_amount.to_string()}
                                        }
                                    }
                                    div { class: "stat",
                                        span { class: "stat-label", "Återstår" }
                                        span { class: "stat-value stat-remaining",
                                            {overview.remaining_budget.to_string()}
                                        }
                                    }
                                }
                            }
            }
        }
