use crate::budget::PeriodFilter;
use crate::budget::budget_hero::BudgetState;
use crate::{Button, ButtonVariant, Input};
use api::models::{BankTransaction, Periodicity};
use api::{create_tag, search_transactions, tag_transaction};
use dioxus::prelude::*;
use uuid::Uuid;

const RETAG_CSS: Asset = asset!("assets/styling/retag-transactions.css");
const PAGE_SIZE: usize = 100;

#[component]
pub fn RetagTransactionsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let period_id = budget_signal().period_id;

    let year: Signal<Option<i32>> = use_signal(|| Some(period_id.year));
    let month: Signal<Option<u32>> = use_signal(|| Some(period_id.month));
    let mut search: Signal<String> = use_signal(String::new);
    let mut tag_filter: Signal<Option<Uuid>> = use_signal(|| None);

    let mut transactions: Signal<Vec<BankTransaction>> = use_signal(Vec::new);
    let mut total_count: Signal<usize> = use_signal(|| 0);
    let mut is_loading: Signal<bool> = use_signal(|| true);
    // Bumped on every filter change so a slow, superseded request can't
    // overwrite the result of a newer one landing first.
    let mut request_id: Signal<u64> = use_signal(|| 0);
    // Per-row "create new tag" state
    let mut creating_for_tx: Signal<Option<Uuid>> = use_signal(|| None);
    let mut new_tag_name: Signal<String> = use_signal(String::new);

    use_effect(move || {
        let y = year().unwrap_or(period_id.year);
        let m = month();
        let tag = tag_filter();
        let s = search();
        let this_request = {
            let mut id = request_id.write();
            *id += 1;
            *id
        };
        is_loading.set(true);
        spawn(async move {
            let search_opt = (!s.trim().is_empty()).then(|| s.trim().to_string());
            let result = search_transactions(budget_id, y, m, tag, search_opt, PAGE_SIZE, 0).await;
            if request_id() != this_request {
                return;
            }
            if let Ok(result) = result {
                transactions.set(result.transactions);
                total_count.set(result.total_count);
            }
            is_loading.set(false);
        });
    });

    let mut tags = budget_signal().tags;
    tags.retain(|t| !t.deleted);
    tags.sort_by(|a, b| a.name.cmp(&b.name));

    let available_years: Vec<i32> = {
        let mut years: Vec<i32> = budget_signal()
            .period_summaries
            .iter()
            .map(|s| s.period_id.year)
            .collect();
        years.sort_unstable();
        years.dedup();
        years
    };

    let active_tag_filter = tag_filter();
    let visible = transactions();
    let has_more = visible.len() < total_count();

    rsx! {
        document::Link { rel: "stylesheet", href: RETAG_CSS }
        div { class: "retag-transactions-view",
            div { class: "retag-search-row",
                Input {
                    placeholder: "Sök transaktioner...",
                    value: search(),
                    oninput: move |e: FormEvent| search.set(e.value()),
                }
                select {
                    class: "retag-tag-filter",
                    onchange: move |e| {
                        tag_filter.set(Uuid::parse_str(&e.value()).ok());
                    },
                    option { value: "", "Alla taggar" }
                    for tag in &tags {
                        option {
                            value: "{tag.id}",
                            selected: active_tag_filter == Some(tag.id),
                            "{tag.name}"
                        }
                    }
                }
                PeriodFilter { year, month, available_years }
                span { class: "retag-count", "{total_count()} transaktioner" }
            }

            if is_loading() {
                p { class: "retag-loading", "Laddar..." }
            } else if visible.is_empty() {
                p { class: "retag-empty",
                    if search().trim().is_empty() && active_tag_filter.is_none() {
                        "Inga taggade transaktioner för perioden."
                    } else {
                        "Inga transaktioner matchar filtret."
                    }
                }
            } else {
                div { class: "retag-table",
                    div { class: "retag-header",
                        span { "Datum" }
                        span { "Beskrivning" }
                        span { "Belopp" }
                        span { "Tagg" }
                    }
                    for tx in visible {
                        {
                            let tx_id = tx.id;
                            let current_tag_id = tx.tag_id;
                            let amount_str = tx.amount.to_string();
                            let date_str = tx.date.format("%Y-%m-%d").to_string();
                            let description = tx.description.clone();
                            let amount_pos = tx.amount.is_pos();
                            let tags_row = tags.clone();
                            let is_creating = creating_for_tx() == Some(tx_id);

                            rsx! {
                                div { key: "{tx_id}", class: "retag-row",
                                    span { class: "retag-date", {date_str} }
                                    span { class: "retag-description", title: description.as_str(), {description.as_str()} }
                                    span { class: if amount_pos { "retag-amount positive" } else { "retag-amount negative" }, {amount_str} }
                                    if is_creating {
                                        div { class: "retag-create-tag-row",
                                            input {
                                                class: "retag-new-tag-input",
                                                r#type: "text",
                                                placeholder: "Taggnamn...",
                                                value: "{new_tag_name}",
                                                autofocus: true,
                                                oninput: move |e: FormEvent| new_tag_name.set(e.value()),
                                                onkeydown: move |e: KeyboardEvent| {
                                                    match e.key() {
                                                        Key::Escape => {
                                                            new_tag_name.set(String::new());
                                                            creating_for_tx.set(None);
                                                        }
                                                        Key::Enter => {
                                                            let name = new_tag_name().trim().to_string();
                                                            if name.is_empty() {
                                                                return;
                                                            }
                                                            new_tag_name.set(String::new());
                                                            creating_for_tx.set(None);
                                                            spawn(async move {
                                                                let Ok(updated) = create_tag(
                                                                        budget_id,
                                                                        name.clone(),
                                                                        Periodicity::Monthly,
                                                                        period_id,
                                                                    )
                                                                    .await else {
                                                                    return;
                                                                };
                                                                let Some(new_tag) = updated
                                                                    .tags
                                                                    .iter()
                                                                    .find(|t| t.name == name && !t.deleted)
                                                                    .cloned() else {
                                                                    return;
                                                                };
                                                                consume_context::<BudgetState>().0.set(updated);
                                                                if let Ok(bv) = tag_transaction(
                                                                        budget_id,
                                                                        tx_id,
                                                                        new_tag.id,
                                                                        period_id,
                                                                    )
                                                                    .await
                                                                {
                                                                    let mut txs = transactions();
                                                                    if let Some(t) = txs.iter_mut().find(|t| t.id == tx_id) {
                                                                        t.tag_id = Some(new_tag.id);
                                                                    }
                                                                    transactions.set(txs);
                                                                    consume_context::<BudgetState>().0.set(bv);
                                                                }
                                                            });
                                                        }
                                                        _ => {}
                                                    }
                                                },
                                            }
                                            button {
                                                r#type: "button",
                                                class: "retag-cancel-create",
                                                onclick: move |_| {
                                                    new_tag_name.set(String::new());
                                                    creating_for_tx.set(None);
                                                },
                                                "×"
                                            }
                                        }
                                    } else {
                                        select {
                                            class: "retag-tag-select",
                                            onchange: move |e| {
                                                if e.value() == "__new__" {
                                                    new_tag_name.set(String::new());
                                                    creating_for_tx.set(Some(tx_id));
                                                    return;
                                                }
                                                let Ok(new_tag_id) = Uuid::parse_str(&e.value()) else {
                                                    return;
                                                };
                                                let mut txs = transactions();
                                                if let Some(t) = txs.iter_mut().find(|t| t.id == tx_id) {
                                                    t.tag_id = Some(new_tag_id);
                                                }
                                                transactions.set(txs);
                                                spawn(async move {
                                                    if let Ok(updated) = tag_transaction(budget_id, tx_id, new_tag_id, period_id)
                                                        .await
                                                    {
                                                        consume_context::<BudgetState>().0.set(updated);
                                                    }
                                                });
                                            },
                                            for tag in &tags_row {
                                                option {
                                                    value: "{tag.id}",
                                                    selected: current_tag_id == Some(tag.id),
                                                    "{tag.name}"
                                                }
                                            }
                                            option { value: "__new__", "＋ Ny tagg..." }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if has_more {
                    Button {
                        variant: ButtonVariant::Secondary,
                        r#type: "button",
                        onclick: move |_| async move {
                            let current_offset = transactions().len();
                            let search_opt = {
                                let s = search();
                                (!s.trim().is_empty()).then(|| s.trim().to_string())
                            };
                            if let Ok(result) = search_transactions(
                                    budget_id,
                                    year().unwrap_or(period_id.year),
                                    month(),
                                    tag_filter(),
                                    search_opt,
                                    PAGE_SIZE,
                                    current_offset,
                                )
                                .await
                            {
                                let mut all = transactions();
                                all.extend(result.transactions);
                                transactions.set(all);
                                total_count.set(result.total_count);
                            }
                        },
                        "Visa fler"
                    }
                }
            }
        }
    }
}
