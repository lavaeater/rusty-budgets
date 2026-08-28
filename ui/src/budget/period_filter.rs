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

/// Year + month (or "Hela året") selector shared by Rapporter and the
/// transaction search. `year` of `None` means "Alla tider" (all time); only
/// offered when `allow_all_time` is set.
#[component]
pub fn PeriodFilter(
    mut year: Signal<Option<i32>>,
    mut month: Signal<Option<u32>>,
    available_years: Vec<i32>,
    #[props(default)] allow_all_time: bool,
) -> Element {
    let years = if available_years.is_empty() {
        year().into_iter().collect()
    } else {
        available_years
    };

    rsx! {
        div { class: "period-filters",
            label { class: "period-filter-field",
                span { "År" }
                select {
                    class: "period-filter-select",
                    value: year().map(|y| y.to_string()).unwrap_or_else(|| "all".to_string()),
                    onchange: move |e| {
                        if e.value() == "all" {
                            year.set(None);
                            month.set(None);
                        } else if let Ok(y) = e.value().parse::<i32>() {
                            year.set(Some(y));
                        }
                    },
                    if allow_all_time {
                        option { value: "all", selected: year().is_none(), "Alla tider" }
                    }
                    for y in years {
                        option { value: y.to_string(), selected: year() == Some(y), "{y}" }
                    }
                }
            }
            if year().is_some() {
                label { class: "period-filter-field",
                    span { "Period" }
                    select {
                        class: "period-filter-select",
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
        }
    }
}
