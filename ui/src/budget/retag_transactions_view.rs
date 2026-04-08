use api::models::BankTransaction;
use api::{get_tagged_transactions, tag_transaction};
use crate::budget::budget_hero::BudgetState;
use crate::{Button, ButtonVariant, Input};
use dioxus::prelude::*;
use uuid::Uuid;

const RETAG_CSS: Asset = asset!("assets/styling/retag-transactions.css");
const PAGE_SIZE: usize = 50;

#[component]
pub fn RetagTransactionsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let period_id = budget_signal().period_id;

    let mut transactions: Signal<Vec<BankTransaction>> = use_signal(Vec::new);
    let mut offset: Signal<usize> = use_signal(|| 0);
    let mut has_more: Signal<bool> = use_signal(|| false);
    let mut search: Signal<String> = use_signal(String::new);
    let mut is_loading: Signal<bool> = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(batch) = get_tagged_transactions(budget_id, PAGE_SIZE + 1, 0).await {
                has_more.set(batch.len() > PAGE_SIZE);
                transactions.set(batch.into_iter().take(PAGE_SIZE).collect());
                offset.set(PAGE_SIZE);
            }
            is_loading.set(false);
        });
    });

    let mut tags = budget_signal().tags;
    tags.retain(|t| !t.deleted);
    tags.sort_by(|a, b| a.name.cmp(&b.name));

    let search_str = search().to_lowercase();
    let visible: Vec<BankTransaction> = if search_str.is_empty() {
        transactions()
    } else {
        transactions()
            .into_iter()
            .filter(|tx| tx.description.to_lowercase().contains(&search_str))
            .collect()
    };

    rsx! {
        document::Link { rel: "stylesheet", href: RETAG_CSS }
        div { class: "retag-transactions-view",
            div { class: "retag-search-row",
                Input {
                    placeholder: "Sök transaktioner...",
                    value: search(),
                    oninput: move |e: FormEvent| search.set(e.value()),
                }
                span { class: "retag-count",
                    "{visible.len()} transaktioner"
                    if has_more() && search_str.is_empty() {
                        " (fler finns)"
                    }
                }
            }

            if is_loading() {
                p { class: "retag-loading", "Laddar..." }
            } else if visible.is_empty() {
                p { class: "retag-empty",
                    if search_str.is_empty() {
                        "Inga taggade transaktioner."
                    } else {
                        "Inga transaktioner matchar sökningen."
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

                            rsx! {
                                div { key: "{tx_id}", class: "retag-row",
                                    span { class: "retag-date", "{date_str}" }
                                    span { class: "retag-description", title: "{description}", "{description}" }
                                    span {
                                        class: if amount_pos { "retag-amount positive" } else { "retag-amount negative" },
                                        "{amount_str}"
                                    }
                                    select {
                                        class: "retag-tag-select",
                                        onchange: move |e| {
                                            let Ok(new_tag_id) = Uuid::parse_str(&e.value()) else { return; };
                                            let mut txs = transactions();
                                            if let Some(t) = txs.iter_mut().find(|t| t.id == tx_id) {
                                                t.tag_id = Some(new_tag_id);
                                            }
                                            transactions.set(txs);
                                            spawn(async move {
                                                if let Ok(updated) = tag_transaction(budget_id, tx_id, new_tag_id, period_id).await {
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
                                    }
                                }
                            }
                        }
                    }
                }

                if has_more() && search_str.is_empty() {
                    Button {
                        variant: ButtonVariant::Secondary,
                        r#type: "button",
                        onclick: move |_| async move {
                            let current_offset = offset();
                            if let Ok(batch) = get_tagged_transactions(budget_id, PAGE_SIZE + 1, current_offset).await {
                                let more = batch.len() > PAGE_SIZE;
                                let new_txs: Vec<_> = batch.into_iter().take(PAGE_SIZE).collect();
                                let mut all = transactions();
                                all.extend(new_txs);
                                transactions.set(all);
                                offset.set(current_offset + PAGE_SIZE);
                                has_more.set(more);
                            }
                        },
                        "Visa fler"
                    }
                }
            }
        }
    }
}
