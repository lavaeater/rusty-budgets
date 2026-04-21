use crate::budget::budget_hero::BudgetState;
use crate::{Button, ButtonVariant, Input};
use api::models::{BankTransaction, Periodicity};
use api::{
    create_tag, get_untagged_transactions, ignore_transaction, preview_rule_matches,
    tag_transaction, update_rule,
};
use dioxus::prelude::*;
use uuid::Uuid;

const TAG_TX_CSS: Asset = asset!("assets/styling/tag-transactions.css");
const BATCH_SIZE: usize = 10;

#[component]
pub fn TagTransactionsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget = budget_signal();
    let budget_id = budget.id;
    let period_id = budget.period_id;

    let mut untagged_txs: Signal<Vec<BankTransaction>> = use_signal(Vec::new);
    let mut current_index: Signal<usize> = use_signal(|| 0);
    let mut selected_tag_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut new_tag_name: Signal<String> = use_signal(String::new);
    let mut new_tag_periodicity: Signal<Periodicity> = use_signal(|| Periodicity::OneOff);
    let mut preview_count: Signal<usize> = use_signal(|| 0);
    let mut tagged_rule_id: Signal<Option<Uuid>> = use_signal(|| None);
    let mut rule_tokens: Signal<Vec<String>> = use_signal(Vec::new);

    use_effect(move || {
        spawn(async move {
            if let Ok(txs) = get_untagged_transactions(budget_id, BATCH_SIZE).await {
                untagged_txs.set(txs);
            }
        });
    });

    let txs = untagged_txs();
    let batch_size = txs.len();
    let total_remaining = budget_signal().untagged_transaction_count;
    let current_tx: Option<BankTransaction> = txs.into_iter().nth(current_index());
    let mut tags = budget_signal()
        .tags
        .into_iter()
        .filter(|t| !t.deleted)
        .collect::<Vec<_>>();

    tags.sort_by(|a, b| a.name.cmp(&b.name));

    // Helper closures for resetting state and re-fetching the next batch
    let mut reset_ui_state = move || {
        selected_tag_id.set(None);
        tagged_rule_id.set(None);
        rule_tokens.set(Vec::new());
        preview_count.set(0);
    };

    rsx! {
        document::Link { rel: "stylesheet", href: TAG_TX_CSS }
        div { class: "tag-transactions-view",
            if total_remaining == 0 && batch_size == 0 {
                div { class: "tag-tx-all-done",
                    p { class: "success-message", "✓ Alla transaktioner är taggade!" }
                }
            } else if current_index() >= batch_size {
                // Exhausted current batch — more may remain
                div { class: "tag-tx-all-done",
                    if total_remaining > 0 {
                        p {
                            "{total_remaining} transaktioner kvar — hoppa vidare till nästa omgång."
                        }
                        Button {
                            r#type: "button",
                            onclick: move |_| async move {
                                if let Ok(txs) = get_untagged_transactions(budget_id, BATCH_SIZE).await {
                                    untagged_txs.set(txs);
                                    current_index.set(0);
                                }
                            },
                            "Nästa omgång"
                        }
                    } else {
                        p { class: "success-message", "✓ Klart! Alla transaktioner genomgångna." }
                    }
                }
            } else if let Some(tx) = current_tx {
                {
                    let tx_id = tx.id;
                    let tx_amount_pos = tx.amount.is_pos();
                    let amount_str = tx.amount.to_string();
                    let date_str = tx.date.format("%Y-%m-%d").to_string();
                    rsx! {
                        div { class: "tag-tx-progress",
                            span {
                                "Transaktion {current_index() + 1}/{batch_size} i omgången"
                                if total_remaining > 0 {
                                    " · {total_remaining} kvar totalt"
                                }
                            }
                            div { class: "tag-tx-progress-bar",
                                div {
                                    class: "tag-tx-progress-fill",
                                    style: "width: {(current_index() + 1) * 100 / batch_size.max(1)}%",
                                }
                            }
                        }

                        div { class: "tag-tx-card",
                            div { class: "tag-tx-description",
                                strong { "{tx.description}" }
                            }
                            div { class: "tag-tx-meta",
                                span { class: "tag-tx-date", "{date_str}" }
                                span { class: if tx_amount_pos { "tag-tx-amount positive" } else { "tag-tx-amount negative" },
                                    "{amount_str}"
                                }
                            }
                        }

                        div { class: "tag-tx-selector",
                            h3 { class: "tag-tx-section-title", "Välj tagg" }
                            div { class: "tag-chips",
                                for tag in tags {
                                    {
                                        let tag_id = tag.id;
                                        let is_selected = selected_tag_id() == Some(tag_id);
                                        rsx! {
                                            span {
                                                key: "{tag_id}",
                                                class: if is_selected { "tag-chip tag-chip-selected" } else { "tag-chip" },
                                                onclick: move |_| async move {
                                                    selected_tag_id.set(Some(tag_id));
                                                    tagged_rule_id.set(None);
                                                    rule_tokens.set(Vec::new());
                                                    if let Ok(matches) = preview_rule_matches(budget_id, tx_id).await {
                                                        preview_count.set(matches.len());
                                                    }
                                                },
                                                "{tag.name}"
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "tag-tx-new-tag-row",
                                Input {
                                    placeholder: "Ny tagg...",
                                    value: new_tag_name(),
                                    oninput: move |e: FormEvent| new_tag_name.set(e.value()),
                                }
                                select {
                                    class: "tag-tx-periodicity-select",
                                    onchange: move |e: FormEvent| {
                                        new_tag_periodicity
                                            .set(
                                                match e.value().as_str() {
                                                    "Quarterly" => Periodicity::Quarterly,
                                                    "Annual" => Periodicity::Annual,
                                                    "OneOff" => Periodicity::OneOff,
                                                    _ => Periodicity::Monthly,
                                                },
                                            );
                                    },
                                    option { value: "Monthly", "Månadsvis" }
                                    option { value: "Quarterly", "Kvartalsvis" }
                                    option { value: "Annual", "Årsvis" }
                                    option { value: "OneOff", selected: true, "Engångskostnad" }
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    r#type: "button",
                                    onclick: move |_| async move {
                                        let name = new_tag_name().trim().to_string();
                                        if name.is_empty() {
                                            return;
                                        }
                                        if let Ok(updated_budget) = create_tag(
                                                budget_id,
                                                name.clone(),
                                                new_tag_periodicity(),
                                                period_id,
                                            )
                                            .await
                                        {
                                            new_tag_name.set(String::new());
                                            if let Some(new_tag) = updated_budget
                                                .tags
                                                .iter()
                                                .find(|t| t.name == name && !t.deleted)
                                            {
                                                let new_id = new_tag.id;
                                                selected_tag_id.set(Some(new_id));
                                                tagged_rule_id.set(None);
                                                rule_tokens.set(Vec::new());
                                                if let Ok(matches) = preview_rule_matches(budget_id, tx_id).await {
                                                    preview_count.set(matches.len());
                                                }
                                            }
                                            consume_context::<BudgetState>().0.set(updated_budget);
                                        }
                                    },
                                    "+"
                                }
                            }
                        }

                        if selected_tag_id().is_some() && tagged_rule_id().is_none() {
                            div { class: "tag-tx-preview",
                                if preview_count() == 0 {
                                    span { "Inga andra transaktioner matchar denna regel" }
                                } else {
                                    span { "{preview_count()} andra transaktioner matchar denna regel" }
                                }
                            }
                        }

                        if tagged_rule_id().is_some() {
                            div { class: "tag-tx-rule-editor",
                                h3 { class: "tag-tx-section-title", "Regelns matchningstermer" }
                                div { class: "tag-tx-rule-tokens",
                                    for (i, token) in rule_tokens().into_iter().enumerate() {
                                        span { key: "{i}", class: "tag-tx-rule-token",
                                            "{token}"
                                            button {
                                                r#type: "button",
                                                class: "tag-tx-token-remove",
                                                onclick: move |_| {
                                                    let mut toks = rule_tokens();
                                                    if i < toks.len() {
                                                        toks.remove(i);
                                                    }
                                                    rule_tokens.set(toks);
                                                },
                                                "×"
                                            }
                                        }
                                    }
                                }
                                if let Some(rule_id) = tagged_rule_id() {
                                    Button {
                                        variant: ButtonVariant::Secondary,
                                        r#type: "button",
                                        onclick: move |_| async move {
                                            if let Ok(bv) = update_rule(budget_id, rule_id, rule_tokens(), period_id).await {
                                                consume_context::<BudgetState>().0.set(bv);
                                            }
                                        },
                                        "Spara & matcha fler transaktioner"
                                    }
                                }
                            }
                        }

                        div { class: "tag-tx-actions",
                            Button {
                                r#type: "button",
                                disabled: selected_tag_id().is_none(),
                                onclick: move |_| async move {
                                    let Some(tag_id) = selected_tag_id() else {
                                        return;
                                    };
                                    let rules_before = budget_signal().match_rules.clone();
                                    if let Ok(updated_budget) = tag_transaction(budget_id, tx_id, tag_id, period_id)
                                        .await
                                    {
                                        let new_rule = updated_budget
                                            .match_rules
                                            .iter()
                                            .find(|r| !rules_before.iter().any(|old| old.id == r.id))
                                            .cloned();
                                        if let Some(rule) = new_rule {
                                            rule_tokens.set(rule.transaction_key.clone());
                                            tagged_rule_id.set(Some(rule.id));
                                        }
                                        consume_context::<BudgetState>().0.set(updated_budget);
                                        selected_tag_id.set(None);
                                        preview_count.set(0);
                                        current_index.set(0);
                                        if let Ok(txs) = get_untagged_transactions(budget_id, BATCH_SIZE).await {
                                            untagged_txs.set(txs);
                                        }
                                    }
                                },
                                "Tagga"
                            }
                            Button {
                                r#type: "button",
                                "data-style": "ghost",
                                onclick: move |_| {
                                    reset_ui_state();
                                    current_index.set(current_index() + 1);
                                },
                                "Hoppa över"
                            }
                            Button {
                                r#type: "button",
                                "data-style": "destructive",
                                onclick: move |_| async move {
                                    if let Ok(bv) =
                                        ignore_transaction(budget_id, tx_id, period_id).await
                                    {
                                        reset_ui_state();
                                        consume_context::<BudgetState>().0.set(bv);
                                        current_index.set(0);
                                        if let Ok(txs) = get_untagged_transactions(budget_id, BATCH_SIZE).await {
                                            untagged_txs.set(txs);
                                        }
                                    }
                                },
                                "Ignorera"
                            }
                        }
                    }
                }
            }
        }
    }
}
