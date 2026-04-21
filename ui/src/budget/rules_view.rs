use api::{delete_rule, update_rule};
use crate::budget::budget_hero::BudgetState;
use crate::Input;
use dioxus::prelude::*;
use uuid::Uuid;

const RULES_CSS: Asset = asset!("assets/styling/rules.css");

#[component]
pub fn RulesView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let budget_id = budget_signal().id;
    let period_id = budget_signal().period_id;

    let mut search: Signal<String> = use_signal(String::new);
    let mut adding_to_rule: Signal<Option<Uuid>> = use_signal(|| None);
    let mut new_token_value: Signal<String> = use_signal(String::new);

    let budget = budget_signal();
    let mut tags = budget.tags.clone();
    tags.sort_by(|a, b| a.name.cmp(&b.name));

    let search_str = search().to_lowercase();

    // Group rules by tag_id, filtered by search
    let mut groups: Vec<(Option<Uuid>, String, Vec<_>)> = {
        let mut map: std::collections::HashMap<Option<Uuid>, Vec<_>> = std::collections::HashMap::new();
        for rule in &budget.match_rules {
            let tag_name = rule
                .tag_id
                .and_then(|tid| tags.iter().find(|t| t.id == tid))
                .map(|t| t.name.as_str())
                .unwrap_or("(utan tagg)");
            // Filter: keep if tag name or any token matches search
            let matches_search = search_str.is_empty()
                || tag_name.to_lowercase().contains(&search_str)
                || rule.transaction_key.iter().any(|tok| tok.contains(&search_str));
            if matches_search {
                map.entry(rule.tag_id).or_default().push(rule.clone());
            }
        }
        map.into_iter()
            .map(|(tag_id, rules)| {
                let tag_name = tag_id
                    .and_then(|tid| tags.iter().find(|t| t.id == tid))
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| "(utan tagg)".to_string());
                (tag_id, tag_name, rules)
            })
            .collect()
    };
    groups.sort_by(|a, b| a.1.cmp(&b.1));

    let total_rules = budget.match_rules.len();

    rsx! {
        document::Link { rel: "stylesheet", href: RULES_CSS }
        div { class: "rules-view",
            div { class: "rules-search-row",
                Input {
                    placeholder: "Sök regler eller taggar...",
                    value: search(),
                    oninput: move |e: FormEvent| search.set(e.value()),
                }
                span { class: "rules-count", "{total_rules} regler" }
            }

            if groups.is_empty() {
                p { class: "rules-empty",
                    if search_str.is_empty() {
                        "Inga regler."
                    } else {
                        "Inga regler matchar sökningen."
                    }
                }
            } else {
                div { class: "rules-groups",
                    for (_tag_id, tag_name, rules) in groups {
                        div { class: "rules-group",
                            div { class: "rules-group-header",
                                span { class: "rules-tag-badge", "{tag_name}" }
                                span { class: "rules-group-count", "{rules.len()}" }
                            }
                            div { class: "rules-group-body",
                                for rule in rules {
                                    {
                                        let rule_id = rule.id;
                                        let tokens = rule.transaction_key.clone();
                                        let is_adding = adding_to_rule() == Some(rule_id);

                                        rsx! {
                                            div { class: "rules-rule-row", key: "{rule_id}",
                                                div { class: "rules-tokens",
                                                    for (i, token) in tokens.iter().enumerate() {
                                                        span { key: "{i}", class: "tag-tx-rule-token",
                                                            "{token}"
                                                            button {
                                                                r#type: "button",
                                                                class: "tag-tx-token-remove",
                                                                title: "Ta bort token",
                                                                onclick: move |_| {
                                                                    // Read latest tokens from state at click time (avoids capturing Vec)
                                                                    let mut toks = consume_context::<BudgetState>()
                                                                        .0()
                                                                        .match_rules
                                                                        .iter()
                                                                        .find(|r| r.id == rule_id)
                                                                        .map(|r| r.transaction_key.clone())
                                                                        .unwrap_or_default();
                                                                    if i < toks.len() {
                                                                        toks.remove(i);
                                                                    }
                                                                    async move {
                                                                        if let Ok(bv) = update_rule(budget_id, rule_id, toks, period_id).await {
                                                                            consume_context::<BudgetState>().0.set(bv);
                                                                        }
                                                                    }
                                                                },
                                                                "×"
                                                            }
                                                        }
                                                    }

                                                    // Add-token inline input
                                                    if is_adding {
                                                        input {
                                                            class: "rules-add-token-input",
                                                            r#type: "text",
                                                            placeholder: "ny token...",
                                                            value: "{new_token_value}",
                                                            autofocus: true,
                                                            oninput: move |e: FormEvent| new_token_value.set(e.value()),
                                                            onkeydown: move |e: KeyboardEvent| {
                                                                let tokens_snap = rule.transaction_key.clone();
                                                                match e.key() {
                                                                    Key::Enter => {
                                                                        let token = new_token_value().trim().to_lowercase();
                                                                        if token.is_empty() {
                                                                            adding_to_rule.set(None);
                                                                            return;
                                                                        }
                                                                        let mut toks = tokens_snap;
                                                                        if !toks.contains(&token) {
                                                                            toks.push(token);
                                                                        }
                                                                        new_token_value.set(String::new());
                                                                        adding_to_rule.set(None);
                                                                        spawn(async move {
                                                                            if let Ok(bv) = update_rule(budget_id, rule_id, toks, period_id)
                                                                                .await
                                                                            {
                                                                                consume_context::<BudgetState>().0.set(bv);
                                                                            }
                                                                        });
                                                                    }
                                                                    Key::Escape => {
                                                                        new_token_value.set(String::new());
                                                                        adding_to_rule.set(None);
                                                                    }
                                                                    _ => {}
                                                                }
                                                            },
                                                            onblur: move |_| {
                                                                new_token_value.set(String::new());
                                                                adding_to_rule.set(None);
                                                            },
                                                        }
                                                    } else {
                                                        button {
                                                            r#type: "button",
                                                            class: "rules-add-token-btn",
                                                            title: "Lägg till token",
                                                            onclick: move |_| {
                                                                new_token_value.set(String::new());
                                                                adding_to_rule.set(Some(rule_id));
                                                            },
                                                            "+"
                                                        }
                                                    }
                                                }

                                                button {
                                                    r#type: "button",
                                                    class: "rules-delete-btn",
                                                    title: "Radera regel",
                                                    onclick: move |_| async move {
                                                        if let Ok(bv) = delete_rule(budget_id, rule_id, period_id).await {
                                                            consume_context::<BudgetState>().0.set(bv);
                                                        }
                                                    },
                                                    "×"
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
        }
    }
}
