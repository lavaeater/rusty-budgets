use api::models::Periodicity;
use api::modify_tag;
use crate::budget::budget_hero::BudgetState;
use crate::Input;
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
                    .map_or(false, |item| item.to_lowercase().contains(&search_str))
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
                    if search_str.is_empty() { "Inga taggar." } else { "Inga taggar matchar sökningen." }
                }
            } else {
                div { class: "tags-table",
                    div { class: "tags-header",
                        span { "Namn" }
                        span { "Periodicitet" }
                        span { "Budgetpost" }
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

                            rsx! {
                                div { key: "{tag_id}", class: "tags-row",
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
                                                    if name.is_empty() || name == original { return; }
                                                    if let Ok(updated) = modify_tag(budget_id, tag_id, Some(name), None, None, period_id).await {
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
                                                if let Ok(updated) = modify_tag(budget_id, tag_id, None, Some(new_p), None, period_id).await {
                                                    consume_context::<BudgetState>().0.set(updated);
                                                }
                                            });
                                        },
                                        option { value: "Monthly",   selected: periodicity == Periodicity::Monthly,   "{periodicity_label(Periodicity::Monthly)}" }
                                        option { value: "Quarterly", selected: periodicity == Periodicity::Quarterly, "{periodicity_label(Periodicity::Quarterly)}" }
                                        option { value: "Annual",    selected: periodicity == Periodicity::Annual,    "{periodicity_label(Periodicity::Annual)}" }
                                        option { value: "OneOff",    selected: periodicity == Periodicity::OneOff,    "{periodicity_label(Periodicity::OneOff)}" }
                                    }

                                    // Budget item connection
                                    if let Some(name) = item_name {
                                        span { class: "tags-item-badge", "{name}" }
                                    } else {
                                        span { class: "tags-item-none", "—" }
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
