use crate::budget::budget_hero::BudgetState;
use crate::budget::classification::{
    COST_KINDS, cost_kind_from_slug, cost_kind_hint, cost_kind_label, cost_kind_slug,
    matching_from_slug, matching_hint, matching_label, matching_slug,
};
use api::models::{CostKind, Matching};
use api::view_models::TagSummary;
use api::{classify_tag, get_tags_needing_review};
use dioxus::prelude::*;
use uuid::Uuid;

const REVIEW_CSS: Asset = asset!("assets/styling/tag-review.css");

/// Guided pass over tags whose classification was inferred rather than chosen.
///
/// `Periodicity::OneOff` was the serde default, so it means "never answered" as
/// often as it means "one-off" — in the production data 34 of 57 tags sit there,
/// mixing real bills (Bredband, Fackförbund) with genuine ad-hoc spending
/// (Shopping, Café). Auto-migrating them would silently switch off
/// auto-categorisation for the bills, so each one is asked about instead, with
/// its actual spend shown alongside.
#[component]
pub fn TagReviewView() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;
    let period_id = budget.period_id;

    let mut pending: Signal<Vec<TagSummary>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);

    use_effect(move || {
        spawn(async move {
            if let Ok(summaries) = get_tags_needing_review(budget_id).await {
                pending.set(summaries);
            }
            loading.set(false);
        });
    });

    if loading() {
        return rsx! {
            document::Link { rel: "stylesheet", href: REVIEW_CSS }
            p { class: "tag-review-loading", "Laddar taggar..." }
        };
    }

    let items = pending();
    if items.is_empty() {
        return rsx! {
            document::Link { rel: "stylesheet", href: REVIEW_CSS }
            div { class: "transactions-section-minimal",
                p { class: "success-message", "✓ Alla taggar är klassificerade!" }
            }
        };
    }

    rsx! {
        document::Link { rel: "stylesheet", href: REVIEW_CSS }
        div { class: "tag-review",
            div { class: "tag-review-intro",
                h4 { "{items.len()} taggar behöver klassificeras" }
                p {
                    "Räkningar taggas automatiskt vid import och periodiseras per månad. "
                    "Rörliga utgifter föreslås för godkännande i stället."
                }
            }
            for summary in items {
                TagReviewRow {
                    key: "{summary.tag_id}",
                    summary: summary.clone(),
                    budget_id,
                    period_id,
                    on_done: move |tag_id: Uuid| {
                        pending.write().retain(|s| s.tag_id != tag_id);
                    },
                }
            }
        }
    }
}

#[component]
fn TagReviewRow(
    summary: TagSummary,
    budget_id: Uuid,
    period_id: api::models::PeriodId,
    on_done: EventHandler<Uuid>,
) -> Element {
    let tag_id = summary.tag_id;
    // Seed from the inferred classification so accepting the default is one click.
    let mut cost_kind = use_signal(|| summary.cost_kind);
    let mut matching = use_signal(|| summary.matching);
    let mut saving = use_signal(|| false);

    let monthly = summary.monthly_budget_contribution;
    let observed = summary.average_monthly;

    rsx! {
        div { class: "tag-review-row",
            div { class: "tag-review-head",
                span { class: "tag-review-name", {summary.name.clone()} }
                span { class: "tag-review-stats",
                    "{summary.transaction_count} transaktioner · "
                    "snitt {observed}/mån"
                }
            }

            div { class: "tag-review-controls",
                label { class: "tag-review-field",
                    span { class: "tag-review-field-label", "Typ av kostnad" }
                    select {
                        class: "tag-review-select",
                        value: cost_kind_slug(cost_kind()),
                        onchange: move |e| {
                            let next = cost_kind_from_slug(&e.value());
                            cost_kind.set(next);
                            // Matching follows the cost kind unless the user
                            // then overrides it explicitly.
                            matching.set(next.default_matching());
                        },
                        for kind in COST_KINDS {
                            option {
                                value: cost_kind_slug(kind),
                                selected: cost_kind() == kind,
                                {cost_kind_label(kind)}
                            }
                        }
                    }
                    span { class: "tag-review-hint", {cost_kind_hint(cost_kind())} }
                }

                label { class: "tag-review-field",
                    span { class: "tag-review-field-label", "Vid import" }
                    select {
                        class: "tag-review-select",
                        value: matching_slug(matching()),
                        onchange: move |e| matching.set(matching_from_slug(&e.value())),
                        for mode in [Matching::Automatic, Matching::Suggest] {
                            option {
                                value: matching_slug(mode),
                                selected: matching() == mode,
                                {matching_label(mode)}
                            }
                        }
                    }
                    span { class: "tag-review-hint", {matching_hint(matching())} }
                }
            }

            if cost_kind().needs_buffer() {
                div { class: "tag-review-periodised",
                    "Periodiseras till "
                    strong { {monthly.to_string()} }
                    " per månad."
                }
            }

            div { class: "tag-review-actions",
                button {
                    class: "tag-review-save",
                    disabled: saving(),
                    onclick: move |_| {
                        saving.set(true);
                        spawn(async move {
                            if let Ok(updated) = classify_tag(
                                    budget_id,
                                    tag_id,
                                    cost_kind(),
                                    matching(),
                                    period_id,
                                )
                                .await
                            {
                                consume_context::<BudgetState>().0.set(updated);
                                on_done.call(tag_id);
                            }
                            saving.set(false);
                        });
                    },
                    if saving() { "Sparar..." } else { "Spara" }
                }
            }
        }
    }
}
