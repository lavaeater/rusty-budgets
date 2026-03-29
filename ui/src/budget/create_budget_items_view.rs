use api::models::{BudgetingType, Periodicity};
use api::view_models::TagSummary;
use api::{create_budget_item, get_unbudgeted_tag_summaries};
use crate::budget::budget_hero::BudgetState;
use crate::{Button, ButtonVariant, Input};
use dioxus::prelude::*;
use uuid::Uuid;

const CREATE_BUDGET_ITEMS_CSS: Asset = asset!("assets/styling/create-budget-items.css");

fn periodicity_label(p: Periodicity) -> &'static str {
    match p {
        Periodicity::Monthly => "Månadsvis",
        Periodicity::Quarterly => "Kvartalsvis",
        Periodicity::Annual => "Årsvis",
        Periodicity::OneOff => "Engångskostnad",
    }
}

fn periodicity_class(p: Periodicity) -> &'static str {
    match p {
        Periodicity::Monthly => "cbi-periodicity-badge",
        Periodicity::Quarterly => "cbi-periodicity-badge quarterly",
        Periodicity::Annual => "cbi-periodicity-badge annual",
        Periodicity::OneOff => "cbi-periodicity-badge oneoff",
    }
}

fn money_class(cents: i64) -> &'static str {
    if cents < 0 { "cbi-money negative" } else if cents > 0 { "cbi-money positive" } else { "cbi-money neutral" }
}

/// Format cents as "X kr" (whole kronor, no decimals — matches Money::Display for SEK).
fn fmt_sek(cents: i64) -> String {
    format!("{} kr", cents / 100)
}

#[component]
pub fn CreateBudgetItemsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    let budget_id = budget.id;
    let period_id = budget.period_id;

    let mut tag_summaries: Signal<Vec<TagSummary>> = use_signal(Vec::new);
    let mut suggested_income_str: Signal<String> = use_signal(String::new);
    let mut selected_tag_ids: Signal<Vec<Uuid>> = use_signal(Vec::new);
    let mut new_item_name: Signal<String> = use_signal(String::new);
    let mut new_item_type: Signal<BudgetingType> = use_signal(|| BudgetingType::Expense);
    let mut is_loading: Signal<bool> = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(summaries) = get_unbudgeted_tag_summaries(budget_id).await {
                tag_summaries.set(summaries);
            }
            is_loading.set(false);
        });
    });

    let summaries = tag_summaries();

    if is_loading() {
        return rsx! {
            document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
            div { class: "create-budget-items-view",
                p { "Laddar taggar..." }
            }
        };
    }

    if summaries.is_empty() {
        return rsx! {
            document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
            div { class: "create-budget-items-done",
                p { class: "success-message", "✓ Alla taggar är budgeterade!" }
            }
        };
    }

    let selected = selected_tag_ids();
    let has_selection = !selected.is_empty();

    // Selected tags' total average monthly (absolute value, always shown as positive cost)
    let selected_monthly_cents: i64 = summaries
        .iter()
        .filter(|s| selected.contains(&s.tag_id))
        .map(|s| s.average_monthly.amount_in_cents())
        .sum();

    // Parse suggested income (in whole kronor, stored as cents)
    let suggested_income_cents: Option<i64> = suggested_income_str()
        .trim()
        .replace(' ', "")
        .parse::<i64>()
        .ok()
        .map(|kr| kr * 100);

    // Total already-budgeted monthly from existing budget items (via BudgetViewModel)
    // We approximate from tag summaries that are NOT in the unbudgeted list — i.e., already budgeted.
    // For now we just show the selection total.

    let remaining_cents = suggested_income_cents.map(|income| income + selected_monthly_cents);

    rsx! {
        document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
        div { class: "create-budget-items-view",

            // --- Suggested income input ---
            div { class: "cbi-income-row",
                span { class: "cbi-income-label", "Föreslagen månadsinkomst:" }
                input {
                    class: "cbi-income-input",
                    r#type: "text",
                    inputmode: "numeric",
                    placeholder: "ex. 35000",
                    value: "{suggested_income_str}",
                    oninput: move |e| suggested_income_str.set(e.value()),
                }
                span { "kr" }
                if let Some(rem) = remaining_cents {
                    span {
                        class: if rem >= 0 { "cbi-income-remaining" } else { "cbi-income-remaining over-budget" },
                        "{fmt_sek(rem)} kvar"
                    }
                }
            }

            // --- Unbudgeted tag table ---
            div { class: "cbi-tag-table",
                div { class: "cbi-tag-header",
                    div {}
                    div { "Tagg" }
                    div { "Periodicitet" }
                    div { "Snitt / mån" }
                    div { "Snitt / år" }
                }
                for summary in summaries.iter() {
                    {
                        let tag_id = summary.tag_id;
                        let is_selected = selected.contains(&tag_id);
                        let monthly = summary.average_monthly.amount_in_cents();
                        let yearly = summary.average_yearly.amount_in_cents();
                        rsx! {
                            div {
                                class: if is_selected { "cbi-tag-row selected" } else { "cbi-tag-row" },
                                onclick: move |_| {
                                    let mut ids = selected_tag_ids();
                                    if ids.contains(&tag_id) {
                                        ids.retain(|id| *id != tag_id);
                                    } else {
                                        ids.push(tag_id);
                                    }
                                    selected_tag_ids.set(ids);
                                },
                                input {
                                    class: "cbi-tag-checkbox",
                                    r#type: "checkbox",
                                    checked: is_selected,
                                    // clicking the row handles toggle; prevent double-fire
                                    onclick: move |e| e.stop_propagation(),
                                    onchange: move |_| {
                                        let mut ids = selected_tag_ids();
                                        if ids.contains(&tag_id) {
                                            ids.retain(|id| *id != tag_id);
                                        } else {
                                            ids.push(tag_id);
                                        }
                                        selected_tag_ids.set(ids);
                                    },
                                }
                                span { class: "cbi-tag-name", "{summary.name}" }
                                span { class: "{periodicity_class(summary.periodicity)}",
                                    "{periodicity_label(summary.periodicity)}"
                                }
                                span { class: "{money_class(monthly)}", "{fmt_sek(monthly)}" }
                                span { class: "{money_class(yearly)}", "{fmt_sek(yearly)}" }
                            }
                        }
                    }
                }
            }

            // --- Create item panel (shown when tags are selected) ---
            if has_selection {
                div { class: "cbi-create-panel",
                    p { class: "cbi-create-panel-title", "Ny budgetpost" }
                    div { class: "cbi-create-form",
                        div { class: "cbi-create-field",
                            label { "Namn" }
                            Input {
                                placeholder: "ex. Transport",
                                value: "{new_item_name}",
                                oninput: move |e: FormEvent| new_item_name.set(e.value()),
                            }
                        }
                        div { class: "cbi-create-field",
                            label { "Typ" }
                            select {
                                class: "cbi-type-select",
                                onchange: move |e| {
                                    new_item_type.set(match e.value().as_str() {
                                        "Income" => BudgetingType::Income,
                                        "Savings" => BudgetingType::Savings,
                                        "InternalTransfer" => BudgetingType::InternalTransfer,
                                        _ => BudgetingType::Expense,
                                    });
                                },
                                option { value: "Expense", "Utgift" }
                                option { value: "Income", "Inkomst" }
                                option { value: "Savings", "Sparande" }
                                option { value: "InternalTransfer", "Intern överföring" }
                            }
                        }
                        Button {
                            r#type: "button",
                            disabled: new_item_name().trim().is_empty(),
                            onclick: move |_| {
                                let name = new_item_name().trim().to_string();
                                if name.is_empty() { return; }
                                let tag_ids = selected_tag_ids();
                                let item_type = new_item_type();
                                spawn(async move {
                                    if let Ok(updated) = create_budget_item(
                                        budget_id,
                                        name,
                                        item_type,
                                        tag_ids,
                                        period_id,
                                    ).await {
                                        consume_context::<BudgetState>().0.set(updated);
                                        // Refresh unbudgeted list
                                        if let Ok(summaries) = get_unbudgeted_tag_summaries(budget_id).await {
                                            tag_summaries.set(summaries);
                                        }
                                        selected_tag_ids.set(Vec::new());
                                        new_item_name.set(String::new());
                                    }
                                });
                            },
                            "Skapa budgetpost"
                        }
                    }
                    p { class: "cbi-selected-total",
                        "Valt: "
                        strong { "{fmt_sek(selected_monthly_cents.abs())} kr/mån" }
                        " ({} taggar)", selected.len()
                    }
                }
            }
        }
    }
}
