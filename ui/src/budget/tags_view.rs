use crate::Input;
use crate::budget::budget_hero::BudgetState;
use api::models::{Money, Periodicity};
use api::modify_tag;
use dioxus::prelude::*;
use uuid::Uuid;

const TAGS_CSS: Asset = asset!("assets/styling/tags.css");

fn periodicity_label(p: Periodicity) -> &'static str {
    match p {
        Periodicity::Monthly => "Månadsvis",
        Periodicity::Quarterly => "Kvartalsvis",
        Periodicity::Annual => "Årsvis",
        Periodicity::OneOff => "Engångskostnad",
    }
}

#[component]
pub fn TagsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let period_id = budget_signal().period_id;

    let mut search: Signal<String> = use_signal(String::new);
    let mut editing_tag_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut editing_name: Signal<String> = use_signal(String::new);
    let mut expanded_tag_id: Signal<Option<Uuid>> = use_signal(|| None);

    let budget = budget_signal();

    // Build tag_id → budget item name lookup
    let tag_to_item: std::collections::HashMap<Uuid, String> = budget
        .items
        .iter()
        .flat_map(|item| item.tag_ids.iter().map(|tid| (*tid, item.name.clone())))
        .collect();

    let search_str = search().to_lowercase();
    let mut tags: Vec<_> = budget
        .tags
        .iter()
        .filter(|t| !t.deleted)
        .filter(|t| {
            search_str.is_empty()
                || t.name.to_lowercase().contains(&search_str)
                || tag_to_item
                    .get(&t.id)
                    .is_some_and(|item| item.to_lowercase().contains(&search_str))
        })
        .cloned()
        .collect();
    tags.sort_by(|a, b| a.name.cmp(&b.name));

    let total = budget.tags.iter().filter(|t| !t.deleted).count();

    rsx! {
        document::Link { rel: "stylesheet", href: TAGS_CSS }
        div { class: "tags-view",
            div { class: "tags-search-row",
                Input {
                    placeholder: "Sök taggar...",
                    value: search(),
                    oninput: move |e: FormEvent| search.set(e.value()),
                }
                span { class: "tags-count", "{total} taggar" }
            }

            if tags.is_empty() {
                p { class: "tags-empty",
                    if search_str.is_empty() {
                        "Inga taggar."
                    } else {
                        "Inga taggar matchar sökningen."
                    }
                }
            } else {
                div { class: "tags-table",
                    div { class: "tags-header",
                        span { "Namn" }
                        span { "Periodicitet" }
                        span { "Budgetpost" }
                        span {}
                    }
                    for tag in tags {
                        {
                            let tag_id = tag.id;
                            let tag_name = tag.name.clone();
                            let tag_name_escape = tag_name.clone();
                            let tag_name_blur = tag_name.clone();
                            let tag_name_click = tag_name.clone();
                            let periodicity = tag.periodicity;
                            let item_name = tag_to_item.get(&tag_id).cloned();
                            let is_editing = editing_tag_id() == Some(tag_id);
                            let is_expanded = expanded_tag_id() == Some(tag_id);

                            rsx! {
                                div { key: "{tag_id}", class: "tags-row-wrapper",
                                    div { class: "tags-row",
                                        // Inline name editor
                                        if is_editing {
                                            input {
                                                class: "tags-name-input",
                                                r#type: "text",
                                                value: "{editing_name}",
                                                autofocus: true,
                                                onclick: move |e| e.stop_propagation(),
                                                oninput: move |e: FormEvent| editing_name.set(e.value()),
                                                onkeydown: move |e: KeyboardEvent| {
                                                    if e.key() == Key::Escape {
                                                        editing_name.set(tag_name_escape.clone());
                                                        editing_tag_id.set(None);
                                                    }
                                                },
                                                onblur: move |_| {
                                                    let original = tag_name_blur.clone();
                                                    async move {
                                                        let name = editing_name().trim().to_string();
                                                        editing_tag_id.set(None);
                                                        if name.is_empty() || name == original {
                                                            return;
                                                        }
                                                        if let Ok(updated) = modify_tag(
                                                                budget_id,
                                                                tag_id,
                                                                Some(name),
                                                                None,
                                                                None,
                                                                period_id,
                                                            )
                                                            .await
                                                        {
                                                            consume_context::<BudgetState>().0.set(updated);
                                                        }
                                                    }
                                                },
                                            }
                                        } else {
                                            span {
                                                class: "tags-name tags-name-editable",
                                                title: "Klicka för att byta namn",
                                                onclick: move |_| {
                                                    editing_name.set(tag_name_click.clone());
                                                    editing_tag_id.set(Some(tag_id));
                                                },
                                                "{tag_name}"
                                            }
                                        }

                                        // Periodicity editor
                                        select {
                                            class: "tags-periodicity-select",
                                            onchange: move |e| {
                                                let new_p = match e.value().as_str() {
                                                    "Quarterly" => Periodicity::Quarterly,
                                                    "Annual" => Periodicity::Annual,
                                                    "OneOff" => Periodicity::OneOff,
                                                    _ => Periodicity::Monthly,
                                                };
                                                spawn(async move {
                                                    if let Ok(updated) = modify_tag(
                                                            budget_id,
                                                            tag_id,
                                                            None,
                                                            Some(new_p),
                                                            None,
                                                            period_id,
                                                        )
                                                        .await
                                                    {
                                                        consume_context::<BudgetState>().0.set(updated);
                                                    }
                                                });
                                            },
                                            option {
                                                value: "Monthly",
                                                selected: periodicity == Periodicity::Monthly,
                                                "{periodicity_label(Periodicity::Monthly)}"
                                            }
                                            option {
                                                value: "Quarterly",
                                                selected: periodicity == Periodicity::Quarterly,
                                                "{periodicity_label(Periodicity::Quarterly)}"
                                            }
                                            option { value: "Annual", selected: periodicity == Periodicity::Annual,
                                                "{periodicity_label(Periodicity::Annual)}"
                                            }
                                            option { value: "OneOff", selected: periodicity == Periodicity::OneOff,
                                                "{periodicity_label(Periodicity::OneOff)}"
                                            }
                                        }

                                        // Budget item connection
                                        if let Some(name) = item_name {
                                            span { class: "tags-item-badge", "{name}" }
                                        } else {
                                            span { class: "tags-item-none", "—" }
                                        }

                                        // Expand toggle
                                        button {
                                            class: if is_expanded { "tags-expand-btn tags-expand-btn-open" } else { "tags-expand-btn" },
                                            r#type: "button",
                                            title: if is_expanded { "Dölj transaktioner" } else { "Visa transaktioner" },
                                            onclick: move |_| {
                                                if is_expanded {
                                                    expanded_tag_id.set(None);
                                                } else {
                                                    expanded_tag_id.set(Some(tag_id));
                                                }
                                            },
                                            "▾"
                                        }
                                    }

                                    if is_expanded {
                                        TagTransactionsPanel { budget_id, tag_id }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TagTransactionsPanel(budget_id: Uuid, tag_id: Uuid) -> Element {
    let transactions =
        use_resource(move || async move { api::get_transactions_for_tag(budget_id, tag_id).await });

    match &*transactions.read() {
        None => rsx! {
            div { class: "tags-tx-panel",
                span { class: "tags-tx-loading", "Laddar..." }
            }
        },
        Some(Err(e)) => rsx! {
            div { class: "tags-tx-panel",
                span { class: "tags-tx-error", "Fel: {e}" }
            }
        },
        Some(Ok(txs)) => {
            let currency = txs.first().map(|tx| tx.amount.currency());
            let total: Option<Money> = currency.map(|cur| {
                txs.iter()
                    .fold(Money::new_cents(0, cur), |acc, tx| acc + tx.amount)
            });

            rsx! {
                div { class: "tags-tx-panel",
                    if txs.is_empty() {
                        span { class: "tags-tx-empty", "Inga taggade transaktioner." }
                    } else {
                        div { class: "tags-tx-summary",
                            span { class: "tags-tx-count", "{txs.len()} transaktioner" }
                            if let Some(total) = total {
                                span { class: "tags-tx-total",
                                    "Totalt: "
                                    strong { {total.to_string()} }
                                }
                            }
                        }
                        div { class: "tags-tx-list",
                            for tx in txs {
                                div { class: "tags-tx-row", key: "{tx.id}",
                                    span { class: "tags-tx-date",
                                        {tx.date.format("%Y-%m-%d").to_string()}
                                    }
                                    span { class: "tags-tx-desc", {tx.description.clone()} }
                                    span { class: if tx.amount.is_pos() { "tags-tx-amount positive" } else { "tags-tx-amount negative" },
                                        {tx.amount.to_string()}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
