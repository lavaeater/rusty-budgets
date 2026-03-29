use api::models::{BudgetingType, Periodicity};
use api::view_models::TagSummary;
use api::{create_budget_item, get_unbudgeted_tag_summaries, modify_tag};
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

fn periodicity_sort_key(p: Periodicity) -> u8 {
    match p {
        Periodicity::Monthly => 0,
        Periodicity::Quarterly => 1,
        Periodicity::Annual => 2,
        Periodicity::OneOff => 3,
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
    let mut selected_tag_ids: Signal<Vec<Uuid>> = use_signal(Vec::new);
    let mut new_item_name: Signal<String> = use_signal(String::new);
    let mut new_item_type: Signal<BudgetingType> = use_signal(|| BudgetingType::Expense);
    let mut is_loading: Signal<bool> = use_signal(|| true);
    let mut sort_col: Signal<&'static str> = use_signal(|| "name");
    let mut sort_asc: Signal<bool> = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(summaries) = get_unbudgeted_tag_summaries(budget_id).await {
                tag_summaries.set(summaries);
            }
            is_loading.set(false);
        });
    });

    let summaries_raw = tag_summaries();

    if is_loading() {
        return rsx! {
            document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
            div { class: "create-budget-items-view",
                p { "Laddar taggar..." }
            }
        };
    }

    if summaries_raw.is_empty() {
        return rsx! {
            document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
            div { class: "create-budget-items-done",
                p { class: "success-message", "✓ Alla taggar är budgeterade!" }
            }
        };
    }

    // Sort summaries
    let mut summaries = summaries_raw;
    let col = sort_col();
    let asc = sort_asc();
    summaries.sort_by(|a, b| {
        let ord = match col {
            "periodicity" => periodicity_sort_key(a.periodicity).cmp(&periodicity_sort_key(b.periodicity)),
            "monthly" => a.average_monthly.amount_in_cents().cmp(&b.average_monthly.amount_in_cents()),
            "yearly" => a.average_yearly.amount_in_cents().cmp(&b.average_yearly.amount_in_cents()),
            _ => a.name.cmp(&b.name),
        };
        if asc { ord } else { ord.reverse() }
    });

    let selected = selected_tag_ids();
    let has_selection = !selected.is_empty();

    let selected_monthly_cents: i64 = summaries
        .iter()
        .filter(|s| selected.contains(&s.tag_id))
        .map(|s| s.average_monthly.amount_in_cents())
        .sum();

    // Helper: returns header class and onclick that toggles sort
    let header_class = move |col_name: &'static str| {
        if sort_col() == col_name {
            if sort_asc() { "cbi-col-header sorted asc" } else { "cbi-col-header sorted desc" }
        } else {
            "cbi-col-header"
        }
    };

    rsx! {
        document::Link { rel: "stylesheet", href: CREATE_BUDGET_ITEMS_CSS }
        div { class: "create-budget-items-view",

            // --- Unbudgeted tag table ---
            div { class: "cbi-tag-table",
                div { class: "cbi-tag-header",
                    div {}
                    div {
                        class: "{header_class(\"name\")}",
                        onclick: move |_| {
                            if sort_col() == "name" { sort_asc.toggle(); } else { sort_col.set("name"); sort_asc.set(true); }
                        },
                        "Tagg"
                    }
                    div {
                        class: "{header_class(\"periodicity\")}",
                        onclick: move |_| {
                            if sort_col() == "periodicity" { sort_asc.toggle(); } else { sort_col.set("periodicity"); sort_asc.set(true); }
                        },
                        "Periodicitet"
                    }
                    div {
                        class: "{header_class(\"monthly\")}",
                        onclick: move |_| {
                            if sort_col() == "monthly" { sort_asc.toggle(); } else { sort_col.set("monthly"); sort_asc.set(true); }
                        },
                        "Snitt / mån"
                    }
                    div {
                        class: "{header_class(\"yearly\")}",
                        onclick: move |_| {
                            if sort_col() == "yearly" { sort_asc.toggle(); } else { sort_col.set("yearly"); sort_asc.set(true); }
                        },
                        "Snitt / år"
                    }
                }
                for summary in summaries.iter() {
                    {
                        let tag_id = summary.tag_id;
                        let tag_name = summary.name.clone();
                        let is_selected = selected.contains(&tag_id);
                        let monthly = summary.average_monthly.amount_in_cents();
                        let yearly = summary.average_yearly.amount_in_cents();
                        let periodicity = summary.periodicity;
                        rsx! {
                            div {
                                key: "{tag_id}",
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
                                span { class: "cbi-tag-name", "{tag_name}" }
                                // Inline periodicity editor
                                select {
                                    class: "{periodicity_class(periodicity)} cbi-periodicity-select",
                                    onclick: move |e| e.stop_propagation(),
                                    onchange: move |e| {
                                        let new_p = match e.value().as_str() {
                                            "Quarterly" => Periodicity::Quarterly,
                                            "Annual" => Periodicity::Annual,
                                            "OneOff" => Periodicity::OneOff,
                                            _ => Periodicity::Monthly,
                                        };
                                        // Update local signal immediately
                                        let mut sums = tag_summaries();
                                        if let Some(s) = sums.iter_mut().find(|s| s.tag_id == tag_id) {
                                            s.periodicity = new_p;
                                        }
                                        tag_summaries.set(sums);
                                        // Persist to server
                                        spawn(async move {
                                            if let Ok(updated) = modify_tag(budget_id, tag_id, None, Some(new_p), None, period_id).await {
                                                consume_context::<BudgetState>().0.set(updated);
                                            }
                                        });
                                    },
                                    option { value: "Monthly", selected: periodicity == Periodicity::Monthly, "Månadsvis" }
                                    option { value: "Quarterly", selected: periodicity == Periodicity::Quarterly, "Kvartalsvis" }
                                    option { value: "Annual", selected: periodicity == Periodicity::Annual, "Årsvis" }
                                    option { value: "OneOff", selected: periodicity == Periodicity::OneOff, "Engångskostnad" }
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
                        strong { "{fmt_sek(selected_monthly_cents.abs())} / mån" }
                        " — {selected.len()} taggar"
                    }
                }
            }
        }
    }
}
