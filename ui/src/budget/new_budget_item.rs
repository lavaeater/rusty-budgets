use uuid::Uuid;
use dioxus::prelude::*;
use api::models::{Budget, BudgetingType, Currency, Money};
use crate::components::{Input, Button};

#[component]
pub fn NewBudgetItem(budgeting_type: BudgetingType, tx_id: Option<Uuid>, close_signal: Option<Signal<bool>>) -> Element {
    let mut budget_signal = use_context::<Signal<Option<Budget>>>();
    let budget_id = budget_signal().unwrap().id;
    let mut new_item_name = use_signal(|| "".to_string());
    let mut new_item_amount = use_signal(|| Money::new_dollars(0, Currency::SEK));
    
    rsx! {
        div { id: "new_item",
            label { "Ny budgetpost" }
            input {
                r#type: "text",
                placeholder: "Namn",
                oninput: move |e| { new_item_name.set(e.value()) },
            }
            input {
                r#type: "number",
                placeholder: "Belopp",
                oninput: move |e| {
                    new_item_amount
                        .set(Money::new_dollars(e.value().parse().unwrap(), Currency::SEK))
                },
            }
            Button {
                r#type: "button",
                "data-style": "primary",
                onclick: move |_| async move {
                    //Here we need some date-aware magic - if we have a transaction!
                    if let Ok((updated_budget, item_id)) = api::add_item(
                            budget_id,
                            new_item_name(),
                            budgeting_type,
                            new_item_amount(),
                            tx_id,
                        )
                        .await
                    {
                        if let Some(tx_id) = tx_id {
                            if let Ok(updated_budget) = api::connect_transaction(
                                    budget_id,
                                    tx_id,
                                    item_id,
                                )
                                .await
                            {
                                budget_signal.set(Some(updated_budget));

                            }
                        } else {
                            budget_signal.set(Some(updated_budget));
                        }
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