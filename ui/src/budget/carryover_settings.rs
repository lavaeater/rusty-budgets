use crate::budget::budget_hero::BudgetState;
use api::configure_carryover;
use api::models::{MonthBeginsOn, PeriodId};
use chrono::Utc;
use dioxus::prelude::*;

const CARRYOVER_CSS: Asset = asset!("assets/styling/carryover.css");

/// Switches envelope carryover on, from a month you choose.
///
/// Off by default and dated on purpose: the log holds history from before the
/// budget was kept properly, where most `ActualItem`s have
/// `budgeted_amount == 0`. Accumulating across all of it would compound
/// spending-with-no-budget into large meaningless balances, so the start month
/// is a deliberate line in the sand.
#[component]
pub fn CarryoverSettings() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;
    let period_id = budget.period_id;
    let current = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());

    let enabled = budget.carryover_from.is_some();
    let mut busy = use_signal(|| false);
    // Default the picker to the current month — the usual "start from now" choice.
    let mut chosen = use_signal(|| budget.carryover_from.unwrap_or(current));

    // Offer the periods that actually exist, newest first, plus the current
    // month in case no transaction has landed in it yet.
    let mut options: Vec<PeriodId> = budget.period_summaries.iter().map(|s| s.period_id).collect();
    if !options.contains(&current) {
        options.push(current);
    }
    options.sort_unstable();
    options.reverse();

    rsx! {
        document::Link { rel: "stylesheet", href: CARRYOVER_CSS }
        div { class: "carryover",
            p { class: "carryover-intro",
                "Med överföring behåller varje budgetpost sitt saldo mellan månader. "
                "Sparar du 1 000 kr i månaden till hundförsäkringen finns 12 000 kr när "
                "räkningen kommer — i stället för ett underskott. Överskridande följer "
                "också med: en post som gått över budget startar nästa månad på minus."
            }

            if let Some(from) = budget.carryover_from {
                div { class: "carryover-status carryover-on",
                    span { class: "carryover-status-icon", "✓" }
                    span {
                        "Överföring är på från och med "
                        strong { {from.to_string()} }
                        ". Månader före den räknas som noll."
                    }
                }
            } else {
                div { class: "carryover-status carryover-off",
                    span { class: "carryover-status-icon", "○" }
                    span { "Överföring är av — varje månad står för sig själv." }
                }
            }

            div { class: "carryover-controls",
                label { class: "carryover-field",
                    span { class: "carryover-field-label", "Börja överföra från" }
                    select {
                        class: "carryover-select",
                        disabled: busy(),
                        value: chosen().to_string(),
                        onchange: move |e| {
                            if let Ok(p) = e.value().parse::<PeriodId>() {
                                chosen.set(p);
                            }
                        },
                        for option in options {
                            option {
                                value: option.to_string(),
                                selected: chosen() == option,
                                {option.to_string()}
                            }
                        }
                    }
                }

                div { class: "carryover-actions",
                    button {
                        class: "carryover-enable",
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            spawn(async move {
                                if let Ok(updated) = configure_carryover(
                                        budget_id,
                                        Some(chosen()),
                                        period_id,
                                    )
                                    .await
                                {
                                    consume_context::<BudgetState>().0.set(updated);
                                }
                                busy.set(false);
                            });
                        },
                        if enabled { "Ändra startmånad" } else { "Slå på överföring" }
                    }
                    if enabled {
                        button {
                            class: "carryover-disable",
                            disabled: busy(),
                            onclick: move |_| {
                                busy.set(true);
                                spawn(async move {
                                    if let Ok(updated) = configure_carryover(budget_id, None, period_id).await {
                                        consume_context::<BudgetState>().0.set(updated);
                                    }
                                    busy.set(false);
                                });
                            },
                            "Stäng av"
                        }
                    }
                }
            }

            p { class: "carryover-note",
                "Inget skrivs om i historiken — saldot räknas fram vid visning, så du kan "
                "ändra startmånad eller stänga av när som helst."
            }
        }
    }
}
