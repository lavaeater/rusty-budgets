use dioxus::core::Element;
use dioxus::core_macro::component;

#[component]
pub fn bork() -> Element {
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
    match e.value().parse() {
    Ok(v) => {
    new_item_amount
    .set(Money::new_dollars(v, budget.currency));
    },
    _ => {
    new_item_amount
    .set(Money::zero(budget.currency));
    }
    }
    },
    }
    Button {
    r#type: "button",
    "data-style": "primary",
    onclick: move |_| async move {
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
    budget_signal.set(Some(updated_budget))
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