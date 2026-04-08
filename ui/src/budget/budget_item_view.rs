use crate::budget::BudgetItemStatusView;
use crate::budget::budget_hero::BudgetState;
use crate::{Button, ButtonVariant, Input, Slider, SliderRange, SliderThumb, SliderTrack};
use api::{create_tag, ignore_transaction, set_item_buffer, tag_transaction};
use api::models::{BudgetingType, Money, Periodicity};
use api::view_models::BudgetItemStatus;
use api::view_models::BudgetItemViewModel;
use api::view_models::BudgetViewModel;
use api::view_models::*;
use dioxus::logger::tracing;
use dioxus::prelude::*;
use dioxus_primitives::slider::SliderValue;
use lucide_dioxus::Pen;
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub fn BudgetItemView(item: BudgetItemViewModel) -> Element {
    let mut expanded = use_signal(|| false);

    let mut edit_item = use_signal(|| false);
    let item_name = use_signal(|| item.name.clone());
    let mut budgeted_amount = use_signal(|| item.budgeted_amount);
    let mut item_type = use_signal(|| item.budgeting_type);
    let mut item_tags = use_signal(|| item.tag_ids.clone());
    let mut new_tag_name = use_signal(String::new);
    let mut buffer_target_str = use_signal(|| {
        item.buffer_target
            .map(|m| m.amount_in_dollars().to_string())
            .unwrap_or_default()
    });

    // State for selected transactions and retagging
    let mut selected_transactions = use_signal(HashSet::<Uuid>::new);
    let mut show_retag_selector = use_signal(|| false);
    let mut creating_retag_tag = use_signal(|| false);
    let mut new_retag_tag_name = use_signal(String::new);
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let remaining_to_budget = budget_signal()
        .overviews
        .iter()
        .find(|ov| ov.budgeting_type == BudgetingType::Income)
        .unwrap()
        .remaining_budget;
    if expanded() {
        rsx! {
            div { class: "budget-item-expanded", key: "{item.item_id}",
                div {
                    class: "budget-item-expanded-header",
                    onclick: move |_| { expanded.set(false) },
                    div { class: "budget-item-expanded-name",
                        "{item_name()}"
                        if !item.tags.is_empty() {
                            span { class: "budget-item-tags-inline",
                                for tag in item.tags.iter() {
                                    span { class: "tag-chip-small", "{tag}" }
                                }
                            }
                        }
                    }
                    div { class: "budget-item-expanded-amounts",
                        "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    }
                }
                if item.transactions.is_empty() {
                    div { class: "no-transactions", "Inga transaktioner" }
                } else {
                    table { class: "transaction-table",
                        thead {
                            tr {
                                th { class: "checkbox-cell", "" }
                                th { "Beskrivning" }
                                th { "Datum" }
                                th { "Belopp" }
                            }
                        }
                        tbody {
                            {
                                item.transactions
                                    .iter()
                                    .map(|transaction| {
                                        let tx_id = transaction.tx_id;
                                        let is_selected = selected_transactions().contains(&tx_id);
                                        rsx! {
                                            tr { key: "{tx_id}",
                                                td { class: "checkbox-cell",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: is_selected,
                                                        onchange: move |_| {
                                                            let mut selected = selected_transactions();
                                                            if is_selected {
                                                                selected.remove(&tx_id);
                                                            } else {
                                                                selected.insert(tx_id);
                                                            }
                                                            selected_transactions.set(selected);
                                                        },
                                                    }
                                                }
                                                td { {transaction.description.clone()} }
                                                td { {transaction.date.format("%Y-%m-%d").to_string()} }
                                                td { {transaction.amount.to_string()} }
                                            }
                                        }
                                    })
                            }
                        }
                    }
                }
                if !selected_transactions().is_empty() {
                    div { class: "transaction-actions-bar",
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {
                                selected_transactions.set(HashSet::new());
                            },
                            "Avmarkera alla"
                        }
                        Button {
                            variant: ButtonVariant::Destructive,
                            onclick: move |_| async move {
                                let mut updated_budget: Option<BudgetViewModel> = None;
                                let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                for tx_id in selected_ids {
                                    if let Ok(ub) = ignore_transaction(
                                            budget_id,
                                            tx_id,
                                            budget_signal().period_id,
                                        )
                                        .await
                                    {
                                        updated_budget = Some(ub);
                                    } else {
                                        updated_budget = None;
                                        break;
                                    }
                                }
                                if let Some(updated_budget) = updated_budget {
                                    info!("Transactions ignored, budget updated");
                                    selected_transactions.set(HashSet::new());
                                    show_retag_selector.set(false);
                                    consume_context::<BudgetState>().0.set(updated_budget);
                                } else {
                                    error!("Transactions ignored, budget not updated");
                                    selected_transactions.set(HashSet::new());
                                    show_retag_selector.set(false);
                                }
                            },
                            "Ignorera alla"
                        }

                        if !show_retag_selector() {
                            Button {
                                variant: ButtonVariant::Primary,
                                onclick: move |_| {
                                    show_retag_selector.set(true);
                                    creating_retag_tag.set(false);
                                    new_retag_tag_name.set(String::new());
                                },
                                "Ändra tagg"
                            }
                        } else if creating_retag_tag() {
                            div { class: "move-selector-container",
                                span { class: "move-selector-label", "Ny tagg:" }
                                Input {
                                    placeholder: "Taggnamn...",
                                    value: new_retag_tag_name(),
                                    oninput: move |e: FormEvent| new_retag_tag_name.set(e.value()),
                                }
                                Button {
                                    variant: ButtonVariant::Primary,
                                    r#type: "button",
                                    onclick: move |_| async move {
                                        let name = new_retag_tag_name().trim().to_string();
                                        if name.is_empty() { return; }
                                        let Ok(updated) = create_tag(budget_id, name.clone(), Periodicity::Monthly, budget_signal().period_id).await else { return; };
                                        let Some(new_tag) = updated.tags.iter().find(|t| t.name == name && !t.deleted).cloned() else { return; };
                                        consume_context::<BudgetState>().0.set(updated);
                                        let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();
                                        for tx_id in selected_ids {
                                            if let Ok(bv) = tag_transaction(budget_id, tx_id, new_tag.id, budget_signal().period_id).await {
                                                consume_context::<BudgetState>().0.set(bv);
                                            }
                                        }
                                        selected_transactions.set(HashSet::new());
                                        show_retag_selector.set(false);
                                        creating_retag_tag.set(false);
                                        new_retag_tag_name.set(String::new());
                                    },
                                    "Skapa & tagga"
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    r#type: "button",
                                    onclick: move |_| { creating_retag_tag.set(false); },
                                    "Tillbaka"
                                }
                            }
                        } else {
                            div { class: "move-selector-container",
                                span { class: "move-selector-label", "Tagga som:" }
                                select {
                                    class: "retag-tag-select",
                                    onchange: move |e| {
                                        if e.value() == "__new__" {
                                            creating_retag_tag.set(true);
                                            return;
                                        }
                                        let Ok(tag_id) = Uuid::parse_str(&e.value()) else { return; };
                                        let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();
                                        spawn(async move {
                                            for tx_id in selected_ids {
                                                if let Ok(bv) = tag_transaction(budget_id, tx_id, tag_id, budget_signal().period_id).await {
                                                    consume_context::<BudgetState>().0.set(bv);
                                                }
                                            }
                                            selected_transactions.set(HashSet::new());
                                            show_retag_selector.set(false);
                                        });
                                    },
                                    option { value: "", disabled: true, selected: true, "Välj tagg..." }
                                    {
                                        let mut sorted_tags = budget_signal().tags.iter()
                                            .filter(|t| !t.deleted)
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        sorted_tags.sort_by(|a, b| a.name.cmp(&b.name));
                                        sorted_tags.into_iter().map(|tag| rsx! {
                                            option { key: "{tag.id}", value: "{tag.id}", "{tag.name}" }
                                        })
                                    }
                                    option { value: "__new__", "＋ Ny tagg..." }
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    r#type: "button",
                                    onclick: move |_| { show_retag_selector.set(false); },
                                    "Avbryt"
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if edit_item() {
        rsx! {
            div { class: "budget-item-edit", key: "{item.item_id}",
                div { class: "budget-item-edit-header",
                    div { class: "budget-item-edit-title", "{item_name()}" }
                    div { class: "budget-item-edit-amounts",
                        "{item.actual_amount.to_string()} / {budgeted_amount().to_string()}"
                    }
                }

                div { class: "budget-item-edit-form",
                    div { class: "budget-item-edit-field",
                        label { class: "budget-item-edit-label", "Taggar" }
                        div { class: "tag-editor",
                            div { class: "tag-chips",
                                {
                                    let mut sorted_tags = budget_signal().tags.iter()
                                        .filter(|t| !t.deleted)
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    sorted_tags.sort_by(|a, b| {
                                        let a_sel = item_tags().contains(&a.id);
                                        let b_sel = item_tags().contains(&b.id);
                                        b_sel.cmp(&a_sel).then_with(|| a.name.cmp(&b.name))
                                    });
                                    sorted_tags.into_iter()
                                        .map(|tag| {
                                            let tag_id = tag.id;
                                            let is_selected = item_tags().contains(&tag_id);
                                            rsx! {
                                                span {
                                                    class: if is_selected { "tag-chip tag-chip-selected" } else { "tag-chip" },
                                                    key: "{tag_id}",
                                                    onclick: move |_| {
                                                        let mut tags = item_tags();
                                                        if is_selected { tags.retain(|id| *id != tag_id); } else { tags.push(tag_id); }
                                                        item_tags.set(tags);
                                                    },
                                                    "{tag.name}"
                                                }
                                            }
                                        })
                                }
                            }
                            div { class: "tag-add-row",
                                Input {
                                    placeholder: "Ny tagg...",
                                    value: new_tag_name(),
                                    oninput: move |e: FormEvent| new_tag_name.set(e.value()),
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    r#type: "button",
                                    onclick: move |_| async move {
                                        let name = new_tag_name().trim().to_string();
                                        if !name.is_empty() {
                                            if let Ok(updated_budget) = api::create_tag(
                                                budget_id,
                                                name.clone(),
                                                Periodicity::Monthly,
                                                budget_signal().period_id,
                                            ).await {
                                                if let Some(new_tag) = updated_budget.tags.iter().find(|t| t.name == name) {
                                                    let mut tags = item_tags();
                                                    tags.push(new_tag.id);
                                                    item_tags.set(tags);
                                                }
                                                new_tag_name.set(String::new());
                                                consume_context::<BudgetState>().0.set(updated_budget);
                                            }
                                        }
                                    },
                                    "Skapa tagg"
                                }
                            }
                        }
                    }
                    div { class: "budget-item-edit-field",
                        label { class: "budget-item-edit-label", "Typ" }
                        select {
                            class: "budget-item-edit-input",
                            onchange: move |e| {
                                item_type.set(match e.value().as_str() {
                                    "Income" => BudgetingType::Income,
                                    "Savings" => BudgetingType::Savings,
                                    "InternalTransfer" => BudgetingType::InternalTransfer,
                                    _ => BudgetingType::Expense,
                                });
                            },
                            option { value: "Expense", selected: item_type() == BudgetingType::Expense, "Utgift" }
                            option { value: "Income", selected: item_type() == BudgetingType::Income, "Inkomst" }
                            option { value: "Savings", selected: item_type() == BudgetingType::Savings, "Sparande" }
                            option { value: "InternalTransfer", selected: item_type() == BudgetingType::InternalTransfer, "Intern överföring" }
                        }
                    }
                    div { class: "budget-item-edit-field",
                        label { class: "budget-item-edit-label", "Budgeterat belopp" }
                        input {
                            class: "budget-item-edit-input",
                            r#type: "number",
                            value: budgeted_amount().amount_in_dollars(),
                            oninput: move |e| {
                                match e.value().parse() {
                                    Ok(v) => {
                                        budgeted_amount.set(Money::new_dollars(v, budget_signal().currency));
                                    }
                                    _ => {
                                        budgeted_amount.set(Money::zero(budget_signal().currency));
                                    }
                                }
                            },
                        }
                        Slider {
                            value: SliderValue::Single(budgeted_amount().amount_in_dollars() as f64),
                            min: 0.0,
                            max: (budgeted_amount() + remaining_to_budget).amount_in_dollars() as f64,
                            step: 1.0,
                            label: "MONEEYYY",
                            horizontal: true,
                            on_value_change: move |v| {
                                let SliderValue::Single(v) = v;
                                budgeted_amount.set(Money::new_dollars(v as i64, budget_signal().currency));
                            },
                            SliderTrack {
                                SliderRange {}
                                SliderThumb {}
                            }
                        }
                    }
                    div { class: "budget-item-edit-field",
                        label { class: "budget-item-edit-label", "Buffertmål (kr)" }
                        div { class: "buffer-target-field",
                            input {
                                class: "budget-item-edit-input",
                                r#type: "number",
                                min: "0",
                                placeholder: "t.ex. 1200 för årsförsäkring",
                                value: "{buffer_target_str}",
                                oninput: move |e: FormEvent| buffer_target_str.set(e.value()),
                            }
                            if let Some(contrib) = item.required_monthly_contribution {
                                span { class: "buffer-contribution-hint",
                                    "→ {contrib} / mån"
                                }
                            } else if let Ok(v) = buffer_target_str().trim().parse::<i64>() {
                                if v > 0 {
                                    // preview based on current tags' max periodicity
                                    span { class: "buffer-contribution-hint buffer-contribution-preview",
                                        "Ange tagg med periodicitet för att se bidrag/mån"
                                    }
                                }
                            }
                        }
                        p { class: "buffer-target-hint",
                            "Tomt = ingen buffert. Fyll i hela beloppet för perioden (t.ex. hela årsavgiften)."
                        }
                    }
                    div { class: "budget-item-edit-actions",
                        Button {
                            variant: ButtonVariant::Primary,
                            onclick: move |_| async move {
                                let tag_ids = item_tags();
                                // Save buffer target
                                let new_buffer = buffer_target_str().trim().parse::<i64>().ok()
                                    .filter(|&v| v > 0)
                                    .map(|kr| Money::new_dollars(kr, budget_signal().currency));
                                let _ = set_item_buffer(budget_id, item.item_id, new_buffer, budget_signal().period_id).await;
                                let _ = api::modify_item(
                                    budget_id,
                                    item.item_id,
                                    None,
                                    Some(item_type()),
                                    Some(tag_ids),
                                    None,
                                    budget_signal().period_id,
                                ).await;
                                if let Some(item_id) = item.actual_id {
                                    match api::modify_actual(
                                            budget_id,
                                            item_id,
                                            budget_signal().period_id,
                                            Some(budgeted_amount()),
                                            None,
                                        )
                                        .await
                                    {
                                        Ok(updated_budget) => {
                                            consume_context::<BudgetState>().0.set(updated_budget);
                                            edit_item.set(false)
                                        }
                                        Err(_) => {
                                            edit_item.set(false);
                                        }
                                    }
                                } else {
                                    match api::add_actual(
                                            budget_id,
                                            item.item_id,
                                            budgeted_amount(),
                                            budget_signal().period_id,
                                        )
                                        .await
                                    {
                                        Ok(updated_budget) => {
                                            consume_context::<BudgetState>().0.set(updated_budget);
                                            edit_item.set(false)
                                        }
                                        Err(_) => {
                                            edit_item.set(false);
                                        }
                                    }
                                }
                            },
                            "Spara"
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            onclick: move |_| {
                                edit_item.set(false);
                            },
                            "Avbryt"
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "budget-item", key: "{item.item_id}",
                div {
                    class: "budget-item-name",
                    onclick: move |_| { expanded.set(!expanded()) },
                    "{item.name}"
                    if !item.tags.is_empty() {
                        span { class: "budget-item-tags-inline",
                            for tag in item.tags.iter() {
                                span { class: "tag-chip-small", "{tag}" }
                            }
                        }
                    }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    onclick: move |_| { edit_item.set(true) },
                    Pen {}
                }
                BudgetItemStatusView { item: item.clone() }
                div { class: "budget-item-amounts",
                    "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    if let Some(contrib) = item.required_monthly_contribution {
                        span { class: "buffer-badge", title: "Rekommenderat buffertsparande per månad",
                            "🏦 {contrib}/mån"
                        }
                    }
                }
            }
        }
    }
}
