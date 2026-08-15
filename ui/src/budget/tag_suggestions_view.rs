use crate::budget::budget_hero::BudgetState;
use api::models::Money;
use api::view_models::TagSuggestion;
use api::{confirm_tag_suggestions, get_tag_suggestions, tag_transaction};
use dioxus::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

const SUGGESTIONS_CSS: Asset = asset!("assets/styling/tag-suggestions.css");

/// Rule matches against `Matching::Suggest` tags, awaiting confirmation.
///
/// Grouped by the proposed tag rather than listed flat: one import can produce
/// dozens of matches for the same payee, and "18 transaktioner ser ut som
/// Livsmedel — godkänn alla" is one decision instead of eighteen.
#[component]
pub fn TagSuggestionsView() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;
    let period_id = budget.period_id;

    let mut suggestions: Signal<Vec<TagSuggestion>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    // Locally skipped transactions. Deliberately not persisted — skipping means
    // "not now", and the transaction stays untagged, so the suggestion returns
    // on reload. The button says "Hoppa över", not "Avvisa", for that reason.
    let mut skipped: Signal<HashSet<Uuid>> = use_signal(HashSet::new);
    let mut busy = use_signal(|| false);

    // Refetch when the untagged count moves: every suggestion is an untagged
    // transaction, so confirming one shrinks both this list and that count.
    // `use_reactive!` is required because the count is a plain value, not a
    // signal — a bare `use_effect` would capture the first render's number.
    let untagged_count = budget.untagged_transaction_count;
    use_effect(use_reactive!(|untagged_count| {
        if untagged_count == 0 {
            // Nothing untagged means nothing can be suggested; skip the call.
            suggestions.set(Vec::new());
            loading.set(false);
        } else {
            spawn(async move {
                if let Ok(found) = get_tag_suggestions(budget_id).await {
                    suggestions.set(found);
                }
                loading.set(false);
            });
        }
    }));

    if loading() {
        return rsx! {
            document::Link { rel: "stylesheet", href: SUGGESTIONS_CSS }
            p { class: "suggestion-loading", "Laddar förslag..." }
        };
    }

    let visible: Vec<TagSuggestion> = suggestions()
        .into_iter()
        .filter(|s| !skipped().contains(&s.tx_id))
        .collect();

    if visible.is_empty() {
        return rsx! {
            document::Link { rel: "stylesheet", href: SUGGESTIONS_CSS }
            div { class: "transactions-section-minimal",
                p { class: "success-message", "✓ Inga förslag att granska!" }
            }
        };
    }

    let groups = group_by_tag(&visible);
    let total = visible.len();

    rsx! {
        document::Link { rel: "stylesheet", href: SUGGESTIONS_CSS }
        div { class: "suggestions",
            div { class: "suggestions-header",
                p { class: "suggestions-intro",
                    "Reglerna känner igen {total} transaktioner, men taggarna är satta till "
                    "att föreslå i stället för att tagga automatiskt. Granska och godkänn."
                }
                if groups.len() > 1 {
                    button {
                        class: "suggestion-accept-all",
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                if let Ok(updated) = confirm_tag_suggestions(budget_id, None, period_id).await {
                                    consume_context::<BudgetState>().0.set(updated);
                                }
                                busy.set(false);
                            });
                        },
                        "Godkänn alla {total}"
                    }
                }
            }

            for group in groups {
                SuggestionGroup {
                    key: "{group.tag_id}",
                    group: group.clone(),
                    budget_id,
                    period_id,
                    on_skip: move |tx_id| { skipped.write().insert(tx_id); },
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct Group {
    tag_id: Uuid,
    tag_name: String,
    total: Money,
    items: Vec<TagSuggestion>,
}

/// Groups suggestions by proposed tag, largest group first — the biggest group
/// is the one worth deciding about.
fn group_by_tag(suggestions: &[TagSuggestion]) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    for s in suggestions {
        if let Some(g) = groups.iter_mut().find(|g| g.tag_id == s.tag_id) {
            g.total += s.amount;
            g.items.push(s.clone());
        } else {
            groups.push(Group {
                tag_id: s.tag_id,
                tag_name: s.tag_name.clone(),
                total: s.amount,
                items: vec![s.clone()],
            });
        }
    }
    groups.sort_by(|a, b| {
        b.items
            .len()
            .cmp(&a.items.len())
            .then_with(|| a.tag_name.cmp(&b.tag_name))
    });
    groups
}

#[component]
fn SuggestionGroup(
    group: Group,
    budget_id: Uuid,
    period_id: api::models::PeriodId,
    on_skip: EventHandler<Uuid>,
) -> Element {
    let mut expanded = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let tag_id = group.tag_id;
    let count = group.items.len();

    rsx! {
        div { class: "suggestion-group",
            div { class: "suggestion-group-head",
                button {
                    class: "suggestion-group-toggle",
                    onclick: move |_| expanded.toggle(),
                    span { class: "suggestion-group-caret",
                        if expanded() { "▾" } else { "▸" }
                    }
                    span { class: "suggestion-group-count", {count.to_string()} }
                    span { class: "suggestion-group-name",
                        "ser ut som "
                        strong { {group.tag_name.clone()} }
                    }
                    span { class: "suggestion-group-total", {group.total.to_string()} }
                }
                button {
                    class: "suggestion-group-accept",
                    disabled: busy(),
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            if let Ok(updated) = confirm_tag_suggestions(
                                    budget_id,
                                    Some(tag_id),
                                    period_id,
                                )
                                .await
                            {
                                consume_context::<BudgetState>().0.set(updated);
                            }
                            busy.set(false);
                        });
                    },
                    if busy() { "Godkänner..." } else { "Godkänn alla" }
                }
            }

            if expanded() {
                div { class: "suggestion-list",
                    for item in group.items.clone() {
                        SuggestionRow {
                            key: "{item.tx_id}",
                            item: item.clone(),
                            budget_id,
                            period_id,
                            on_skip,
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SuggestionRow(
    item: TagSuggestion,
    budget_id: Uuid,
    period_id: api::models::PeriodId,
    on_skip: EventHandler<Uuid>,
) -> Element {
    let mut busy = use_signal(|| false);
    let tx_id = item.tx_id;
    let tag_id = item.tag_id;
    let date = item.date.format("%Y-%m-%d").to_string();

    rsx! {
        div { class: "suggestion-row",
            span { class: "suggestion-date", {date} }
            span { class: "suggestion-description", title: "{item.description}",
                {item.description.clone()}
            }
            span { class: "suggestion-amount", {item.amount.to_string()} }
            div { class: "suggestion-actions",
                button {
                    class: "suggestion-confirm",
                    disabled: busy(),
                    title: "Godkänn förslaget",
                    onclick: move |_| {
                        busy.set(true);
                        spawn(async move {
                            if let Ok(updated) = tag_transaction(budget_id, tx_id, tag_id, period_id).await {
                                consume_context::<BudgetState>().0.set(updated);
                            }
                            busy.set(false);
                        });
                    },
                    "✓"
                }
                button {
                    class: "suggestion-skip",
                    title: "Hoppa över — transaktionen förblir otaggad",
                    onclick: move |_| on_skip.call(tx_id),
                    "✕"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use api::models::Currency;
    use chrono::Utc;

    fn suggestion(tag: &str, tag_id: Uuid, amount: i64) -> TagSuggestion {
        TagSuggestion {
            tx_id: Uuid::new_v4(),
            tag_id,
            tag_name: tag.to_string(),
            description: format!("{tag} betalning"),
            amount: Money::new_dollars(amount, Currency::SEK),
            date: Utc::now(),
        }
    }

    #[test]
    fn groups_share_a_row_per_tag() {
        let food = Uuid::new_v4();
        let cafe = Uuid::new_v4();
        let groups = group_by_tag(&[
            suggestion("Livsmedel", food, -100),
            suggestion("Café", cafe, -40),
            suggestion("Livsmedel", food, -250),
        ]);

        assert_eq!(groups.len(), 2, "one row per proposed tag");
        assert_eq!(groups[0].tag_id, food, "largest group first");
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(
            groups[0].total,
            Money::new_dollars(-350, Currency::SEK),
            "group total sums its members"
        );
    }

    #[test]
    fn equal_sized_groups_sort_by_name() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let groups = group_by_tag(&[suggestion("Övrigt", a, -10), suggestion("Apotek", b, -10)]);
        assert_eq!(groups[0].tag_name, "Apotek", "stable, alphabetical tie-break");
        assert_eq!(groups[1].tag_name, "Övrigt");
    }

    #[test]
    fn no_suggestions_means_no_groups() {
        assert!(group_by_tag(&[]).is_empty());
    }

    #[test]
    fn a_single_suggestion_still_forms_a_group() {
        let groups = group_by_tag(&[suggestion("Hund", Uuid::new_v4(), -12000)]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].items.len(), 1);
        assert_eq!(groups[0].total, Money::new_dollars(-12000, Currency::SEK));
    }
}
