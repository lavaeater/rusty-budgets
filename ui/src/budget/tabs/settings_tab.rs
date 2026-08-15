use crate::budget::budget_hero::BudgetState;
use crate::budget::{CarryoverSettings, RulesView, TagReviewView, TagsView};
use crate::file_chooser::{FileData, FileDialog};
use crate::Button;
use api::models::{MonthBeginsOn, PeriodId};
use api::{auto_budget_all, auto_budget_period, import_transactions_bytes};
use chrono::Utc;
use dioxus::logger::tracing::info;
use dioxus::prelude::*;

/// Maintenance: import, tags, rules and the auto-budget tools.
///
/// These used to live in the page header and in `<details>` blocks at the bottom
/// of the main page.
#[component]
pub fn SettingsTab() -> Element {
    let budget = use_context::<BudgetState>().0();
    let budget_id = budget.id;
    let period_id = budget.period_id;
    let period_id_now = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());
    let auto_budget_enabled = budget.period_id != period_id_now;

    let import_file = move |file: FileData| {
        let contents = file.contents;
        spawn(async move {
            if !contents.is_empty()
                && let Ok(updated_budget) =
                    import_transactions_bytes(budget_id, contents, period_id).await
            {
                info!("Import went well and we update the context bro");
                consume_context::<BudgetState>().0.set(updated_budget);
            }
        });
    };

    rsx! {
        div { class: "tab-panel",
            section { class: "settings-section",
                h3 { class: "settings-section-title", "Importera transaktioner" }
                FileDialog { on_chosen: import_file }
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Verktyg" }
                div { class: "settings-tools",
                    if auto_budget_enabled {
                        Button {
                            onclick: move |_| async move {
                                if let Ok(bv) = auto_budget_period(budget_id, period_id).await {
                                    consume_context::<BudgetState>().0.set(bv);
                                }
                            },
                            "Auto budget period"
                        }
                    }
                    Button {
                        onclick: move |_| async move {
                            if let Ok(bv) = auto_budget_all(budget_id, period_id).await {
                                consume_context::<BudgetState>().0.set(bv);
                            }
                        },
                        "Auto budget alla perioder"
                    }
                }
            }

            if budget.tags_needing_review_count > 0 {
                section { class: "settings-section",
                    h3 { class: "settings-section-title", "Klassificera taggar" }
                    TagReviewView {}
                }
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Överföring mellan månader" }
                CarryoverSettings {}
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Taggar" }
                TagsView {}
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Taggningsregler" }
                RulesView {}
            }
        }
    }
}
