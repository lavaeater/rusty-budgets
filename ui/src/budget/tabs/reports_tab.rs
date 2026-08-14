use crate::budget::budget_hero::BudgetState;
use dioxus::prelude::*;

/// Period-over-period trends.
///
/// Revives `RunningDeficitView`, which was mothballed in Sprint 1 for being the
/// right data in the wrong place — this is the right place.
#[component]
pub fn ReportsTab() -> Element {
    rsx! {
        div { class: "tab-panel",
            section { class: "report-section",
                h3 { class: "report-section-title", "Löpande över-/underskott" }
                RunningDeficitView {}
            }
        }
    }
}

#[component]
fn RunningDeficitView() -> Element {
    let budget = use_context::<BudgetState>().0();
    let summaries = &budget.period_summaries;
    if summaries.is_empty() {
        return rsx! {
            p { class: "report-empty", "Ingen perioddata ännu." }
        };
    }
    let final_running = summaries.last().map_or_else(Default::default, |s| s.running_net);
    let banner_class = if final_running.amount_in_cents() < 0 {
        "running-banner running-banner-negative"
    } else {
        "running-banner running-banner-positive"
    };
    rsx! {
        div {
            div { class: banner_class,
                if final_running.amount_in_cents() < 0 {
                    "Totalt underskott: {final_running}"
                } else {
                    "Totalt överskott: {final_running}"
                }
            }
            table { class: "report-table",
                thead {
                    tr {
                        th { "Period" }
                        th { class: "num", "Inkomst" }
                        th { class: "num", "Utgifter" }
                        th { class: "num", "Netto" }
                        th { class: "num", "Löpande" }
                    }
                }
                tbody {
                    for summary in summaries.iter().rev().take(24) {
                        tr {
                            td { "{summary.period_id}" }
                            td { class: "num", "{summary.income_actual}" }
                            td { class: "num", "{summary.expense_actual}" }
                            td {
                                class: if summary.net.amount_in_cents() < 0 { "num neg" } else { "num pos" },
                                "{summary.net}"
                            }
                            td {
                                class: if summary.running_net.amount_in_cents() < 0 { "num neg" } else { "num pos" },
                                "{summary.running_net}"
                            }
                        }
                    }
                }
            }
        }
    }
}
