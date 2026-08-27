use crate::budget::PeriodFilter;
use crate::budget::budget_hero::BudgetState;
use api::get_report;
use api::view_models::ReportViewModel;
use dioxus::prelude::*;

/// Budgeted vs. actual per budget item (and per tag within it) for a chosen
/// year, or a chosen month within that year.
#[component]
pub fn ReportsTab() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;

    // `None` means "Alla tider" (all time).
    let year: Signal<Option<i32>> = use_signal(|| Some(budget.period_id.year));
    // `None` means "Hela året".
    let month: Signal<Option<u32>> = use_signal(|| Some(budget.period_id.month));

    let report = use_resource(move || async move { get_report(budget_id, year(), month()).await });

    rsx! {
        div { class: "tab-panel",
            section { class: "report-section",
                h3 { class: "report-section-title", "Rapporter" }

                {
                    let years = report
                        .read()
                        .as_ref()
                        .and_then(|r| r.as_ref().ok())
                        .map(|r| r.available_years.clone())
                        .filter(|years| !years.is_empty())
                        .unwrap_or_else(|| year().into_iter().collect());
                    rsx! {
                        div { class: "report-filters-row",
                            PeriodFilter {
                                year,
                                month,
                                available_years: years,
                                allow_all_time: true,
                            }
                        }
                    }
                }

                match report.read().as_ref() {
                    None => rsx! {
                        p { class: "report-empty", "Laddar..." }
                    },
                    Some(Err(_)) => rsx! {
                        p { class: "report-empty", "Kunde inte ladda rapporten." }
                    },
                    Some(Ok(report)) => rsx! {
                        ReportItems { report: report.clone() }
                    },
                }
            }
        }
    }
}

#[component]
fn ReportItems(report: ReportViewModel) -> Element {
    if report.items.is_empty() {
        return rsx! {
            p { class: "report-empty", "Inga budgetposter ännu." }
        };
    }

    rsx! {
        div { class: "report-items",
            for item in report.items {
                div { class: "report-item-card",
                    div { class: "report-item-header",
                        span { class: "report-item-name", "{item.name}" }
                        span { class: "report-item-type", "{item.budgeting_type}" }
                        span {
                            class: if item.actual_amount.amount_in_cents() > item.budgeted_amount.amount_in_cents() { "report-item-amounts over" } else { "report-item-amounts" },
                            "{item.actual_amount} / {item.budgeted_amount}"
                        }
                    }
                    if !item.tags.is_empty() {
                        table { class: "report-table report-tag-table",
                            tbody {
                                for tag in item.tags {
                                    tr {
                                        td { "{tag.name}" }
                                        td { class: "num", "{tag.actual_amount}" }
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
