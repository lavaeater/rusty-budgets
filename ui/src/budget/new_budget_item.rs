use uuid::Uuid;
use dioxus::prelude::*;
use api::models::{Budget, BudgetingType, Currency, Money};
use api::view_models::BudgetViewModel;
use crate::budget::budget_hero::BudgetState;
use crate::components::{Input, Button};

#[component]
pub fn NewBudgetItem(budgeting_type: BudgetingType, tx_id: Option<Uuid>, close_signal: Option<Signal<bool>>) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));
    
            let budget_id = budget_signal().id;
            let period_id = budget_signal().period_id;

            rsx! {
                div { id: "new_item",
                    label { "Ny budgetpost" }
                    input {
                        r#type: "text",
                        placeholder: "Namn",
                        oninput: move |e| {
                            new_item_name.set(e.value());
                        },
                    }
                    input {
                        r#type: "number",
                        placeholder: "Belopp",
                        oninput: move |e| {
                            match e.value().parse() {
                                Ok(v) => {
                                    new_item_amount.set(Money::new_dollars(v, budget_signal().currency));
                                }
                                _ => {
                                    new_item_amount.set(Money::zero(budget_signal().currency));
                                }
                            }
                        },
                    }
                    Button {
                        r#type: "button",
                        "data-style": "primary",
                        onclick: move |_| async move {
                            info!(
                                "Add New Actual Item with: {}, {}, {}, {:#?}, {}", new_item_name(),
                                budgeting_type, new_item_amount(), tx_id, period_id
                            );
                            if let Ok(updated_budget) = api::add_new_actual_item(
                                    budget_id,
                                    new_item_name(),
                                    budgeting_type,
                                    new_item_amount(),
                                    tx_id,
                                    period_id,
                                )
                                .await
                            {
                                consume_context::<BudgetState>().0.set(updated_budget);
                                if let Some(mut closer) = close_signal {
                                    closer.set(false);
                                }
                            }
                        },
                        "Lägg till"
                    }
                    Button {
                        r#type: "button",
                        "data-style": "outline",
                        onclick: move |_| {
                            if let Some(mut closer) = close_signal {
                                closer.set(false);
                            }
                        },
                        "Avbryt"
                    }
                }
            }
        }