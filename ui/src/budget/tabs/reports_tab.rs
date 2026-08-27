use crate::budget::budget_hero::BudgetState;
use api::get_report;
use api::view_models::ReportViewModel;
use dioxus::prelude::*;

const MONTH_NAMES: [&str; 12] = [
    "Januari",
    "Februari",
    "Mars",
    "April",
    "Maj",
    "Juni",
    "Juli",
    "Augusti",
    "September",
    "Oktober",
    "November",
    "December",
];

/// Budgeted vs. actual per budget item (and per tag within it) for a chosen
/// year, or a chosen month within that year.
#[component]
pub fn ReportsTab() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;

    let mut year = use_signal(|| budget.period_id.year);
    // `None` means "Hela året".
    let mut month: Signal<Option<u32>> = use_signal(|| Some(budget.period_id.month));

    let report = use_resource(move || async move { get_report(budget_id, year(), month()).await });

    rsx! {
        div { class: "tab-panel",
            section { class: "report-section",
                h3 { class: "report-section-title", "Rapporter" }

                div { class: "report-filters",
                    label { class: "report-filter-field",
                        span { "År" }
                        select {
                            class: "report-filter-select",
                            value: year().to_string(),
                            onchange: move |e| {
                                if let Ok(y) = e.value().parse::<i32>() {
                                    year.set(y);
                                }
                            },
                            {
                                let years = report
                                    .read()
                                    .as_ref()
                                    .and_then(|r| r.as_ref().ok())
                                    .map(|r| r.available_years.clone())
                                    .filter(|years| !years.is_empty())
                                    .unwrap_or_else(|| vec![year()]);
                                rsx! {
                                    for y in years {
                                        option { value: y.to_string(), selected: y == year(), "{y}" }
                                    }
                                }
                            }
                        }
                    }
                    label { class: "report-filter-field",
                        span { "Period" }
                        select {
                            class: "report-filter-select",
                            value: month().map(|m| m.to_string()).unwrap_or_else(|| "all".to_string()),
                            onchange: move |e| {
                                if e.value() == "all" {
                                    month.set(None);
                                } else if let Ok(m) = e.value().parse::<u32>() {
                                    month.set(Some(m));
                                }
                            },
                            option { value: "all", selected: month().is_none(), "Hela året" }
                            for (index , name) in MONTH_NAMES.iter().enumerate() {
                                {
                                    let m = index as u32 + 1;
                                    rsx! {
                                        option { value: m.to_string(), selected: month() == Some(m), "{name}" }
                                    }
                                }
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
