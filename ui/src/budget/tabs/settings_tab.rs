use crate::budget::budget_hero::BudgetState;
use crate::budget::{CarryoverSettings, RulesView, TagReviewView, TagsView};
use crate::file_chooser::{FileData, FileDialog};
use crate::{Button, ButtonVariant, Input};
use api::models::{BankAccount, BankAccountType, MonthBeginsOn, PeriodId, format_account_number};
use api::{
    auto_budget_all, auto_budget_period, create_budget, export_budget, export_tags_and_rules,
    import_budget, import_tags_and_rules, import_transactions_bytes, list_budgets,
    modify_bank_account, normalize_account_numbers, switch_budget,
};
use chrono::Utc;
use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use uuid::Uuid;

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

    let mut normalizing_accounts: Signal<bool> = use_signal(|| false);
    let mut normalize_status: Signal<Option<String>> = use_signal(|| None);
    let accounts_before = budget.accounts.len();

    let mut export_tags_and_rules_checked: Signal<bool> = use_signal(|| true);
    let mut export_transfer_rules_checked: Signal<bool> = use_signal(|| true);
    let mut export_bank_accounts_checked: Signal<bool> = use_signal(|| true);

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

    let export_rules = move |_| {
        spawn(async move {
            let Ok(json) = export_tags_and_rules(
                    budget_id,
                    export_tags_and_rules_checked(),
                    export_transfer_rules_checked(),
                    export_bank_accounts_checked(),
                )
                .await
            else {
                info!("Failed to export selected data");
                return;
            };
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Spara valda delar")
                .set_file_name("budget-regler.json")
                .save_file()
                .await;
            if let Some(handle) = handle
                && let Err(e) = handle.write(json.as_bytes()).await
            {
                info!("Failed to save export: {}", e);
            }
        });
    };

    let import_rules = move |file: FileData| {
        let contents = file.contents;
        spawn(async move {
            if !contents.is_empty()
                && let Ok(updated_budget) =
                    import_tags_and_rules(budget_id, contents, period_id).await
            {
                info!("Rules import went well and we update the context bro");
                consume_context::<BudgetState>().0.set(updated_budget);
            }
        });
    };

    let mut budget_import_status: Signal<Option<String>> = use_signal(|| None);

    let export_budget_click = move |_| {
        spawn(async move {
            let Ok(json) = export_budget(budget_id).await else {
                info!("Failed to export budget");
                return;
            };
            let handle = rfd::AsyncFileDialog::new()
                .set_title("Spara hela budgeten")
                .set_file_name("budget-export.json")
                .save_file()
                .await;
            if let Some(handle) = handle
                && let Err(e) = handle.write(json.as_bytes()).await
            {
                info!("Failed to save budget export: {}", e);
            }
        });
    };

    let import_budget_file = move |file: FileData| {
        let contents = file.contents;
        spawn(async move {
            if contents.is_empty() {
                return;
            }
            match import_budget(contents).await {
                Ok(_) => budget_import_status.set(Some(
                    "Budgeten importerades. Starta om appen för att se den, om den inte redan visas.".to_string(),
                )),
                Err(e) => {
                    info!("Failed to import budget: {}", e);
                    budget_import_status.set(Some("Kunde inte importera budgeten.".to_string()));
                }
            }
        });
    };

    let mut budgets_resource = use_resource(move || async move { list_budgets().await });
    let mut new_budget_name: Signal<String> = use_signal(String::new);
    let mut budget_switch_status: Signal<Option<String>> = use_signal(|| None);

    let create_new_budget = move |_| {
        spawn(async move {
            let name = new_budget_name().trim().to_string();
            if name.is_empty() {
                return;
            }
            match create_budget(name, period_id, Some(false)).await {
                Ok(_) => {
                    new_budget_name.set(String::new());
                    budgets_resource.restart();
                }
                Err(e) => {
                    info!("Failed to create budget: {}", e);
                    budget_switch_status.set(Some("Kunde inte skapa budgeten.".to_string()));
                }
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
                h3 { class: "settings-section-title", "Bankkonton" }
                p { class: "settings-hint",
                    "Kontotyp styr hur överföringar mellan konton tolkas — en överföring "
                    "till ett sparkonto räknas alltid som sparande, aldrig som en intern "
                    "float-överföring."
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    r#type: "button",
                    disabled: normalizing_accounts(),
                    onclick: move |_| async move {
                        normalizing_accounts.set(true);
                        match normalize_account_numbers(budget_id, period_id).await {
                            Ok(bv) => {
                                let merged = accounts_before.saturating_sub(bv.accounts.len());
                                normalize_status.set(Some(if merged > 0 {
                                    format!("{merged} dubblettkonton slogs ihop.")
                                } else {
                                    "Inga dubbletter hittades.".to_string()
                                }));
                                consume_context::<BudgetState>().0.set(bv);
                            }
                            Err(e) => {
                                info!("Failed to normalize account numbers: {}", e);
                                normalize_status.set(Some("Kunde inte normalisera kontonummer.".to_string()));
                            }
                        }
                        normalizing_accounts.set(false);
                    },
                    if normalizing_accounts() {
                        "Normaliserar..."
                    } else {
                        "Normalisera kontonummer"
                    }
                }
                if let Some(status) = normalize_status() {
                    p { class: "settings-hint", {status} }
                }
                div { class: "settings-budget-list",
                    for account in budget.accounts.clone() {
                        AccountRow {
                            key: "{account.id}",
                            account,
                            budget_id,
                            period_id,
                        }
                    }
                }
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Taggar" }
                TagsView {}
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Taggningsregler" }
                RulesView {}
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Exportera / importera" }
                p { class: "settings-hint",
                    "Välj vilka delar som ska ingå i exporten. Import läser bara de delar "
                    "filen faktiskt innehåller — övrigt lämnas orört."
                }
                div { class: "settings-export-options",
                    label { class: "settings-checkbox-label",
                        input {
                            r#type: "checkbox",
                            checked: export_tags_and_rules_checked(),
                            onchange: move |e: FormEvent| export_tags_and_rules_checked.set(e.checked()),
                        }
                        "Taggningsregler"
                    }
                    label { class: "settings-checkbox-label",
                        input {
                            r#type: "checkbox",
                            checked: export_transfer_rules_checked(),
                            onchange: move |e: FormEvent| export_transfer_rules_checked.set(e.checked()),
                        }
                        "Överföringsregler"
                    }
                    label { class: "settings-checkbox-label",
                        input {
                            r#type: "checkbox",
                            checked: export_bank_accounts_checked(),
                            onchange: move |e: FormEvent| export_bank_accounts_checked.set(e.checked()),
                        }
                        "Bankkonton (namn och typer)"
                    }
                }
                div { class: "settings-tools",
                    Button { class: "primary", onclick: export_rules, "Exportera till fil" }
                    FileDialog {
                        on_chosen: import_rules,
                        label: "Importera från fil",
                        title: "Välj en JSON-fil att importera",
                        filter_name: "JSON",
                        filter_extensions: vec!["json".to_string()],
                    }
                }
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Exportera / importera hela budgeten" }
                p { class: "settings-hint",
                    "En fullständig kopia av budgeten — konton, taggar, regler, budgetposter, "
                    "perioder och alla transaktioner — för att flytta den till en annan instans "
                    "av appen. Import skapar alltid en ny budget, den ersätter aldrig en befintlig."
                }
                div { class: "settings-tools",
                    Button { class: "primary", onclick: export_budget_click, "Exportera till fil" }
                    FileDialog {
                        on_chosen: import_budget_file,
                        label: "Importera från fil",
                        title: "Välj en JSON-fil med en hel budget",
                        filter_name: "JSON",
                        filter_extensions: vec!["json".to_string()],
                    }
                }
                if let Some(status) = budget_import_status() {
                    p { class: "settings-hint", {status} }
                }
            }

            section { class: "settings-section",
                h3 { class: "settings-section-title", "Budgetar" }
                match &*budgets_resource.read() {
                    None => rsx! {
                        p { class: "settings-hint", "Laddar budgetar..." }
                    },
                    Some(Err(_)) => rsx! {
                        p { class: "settings-hint", "Kunde inte hämta budgetlistan." }
                    },
                    Some(Ok(budgets)) => rsx! {
                        div { class: "settings-budget-list",
                            for b in budgets.clone() {
                                div { key: "{b.id}", class: "settings-budget-row",
                                    span { class: "settings-budget-name", "{b.name}" }
                                    if b.default {
                                        span { class: "settings-budget-active", "Aktiv" }
                                    } else {
                                        Button {
                                            onclick: move |_| {
                                                let budget_id = b.id;
                                                spawn(async move {
                                                    match switch_budget(budget_id, period_id).await {
                                                        Ok(updated) => {
                                                            consume_context::<BudgetState>().0.set(updated);
                                                            budgets_resource.restart();
                                                        }
                                                        Err(e) => {
                                                            info!("Failed to switch budget: {}", e);
                                                            budget_switch_status
                                                                .set(Some("Kunde inte växla budget.".to_string()));
                                                        }
                                                    }
                                                });
                                            },
                                            "Växla"
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
                div { class: "settings-tools",
                    Input {
                        placeholder: "Namn på ny budget",
                        value: new_budget_name(),
                        oninput: move |e: FormEvent| new_budget_name.set(e.value()),
                    }
                    Button { class: "primary", onclick: create_new_budget, "Skapa budget" }
                }
                if let Some(status) = budget_switch_status() {
                    p { class: "settings-hint", {status} }
                }
            }
        }
    }
}

#[component]
fn AccountRow(account: BankAccount, budget_id: Uuid, period_id: PeriodId) -> Element {
    let account_id = account.id;
    let formatted_number = format_account_number(&account.account_number);
    let original_description = account.description.clone();
    let mut description = use_signal(|| account.description.clone());

    let commit_description = move |_| {
        let name = description().trim().to_string();
        if name.is_empty() || name == original_description {
            description.set(original_description.clone());
            return;
        }
        spawn(async move {
            if let Ok(bv) =
                modify_bank_account(budget_id, account_id, None, Some(name), period_id).await
            {
                consume_context::<BudgetState>().0.set(bv);
            }
        });
    };

    rsx! {
        div { class: "settings-budget-row",
            Input {
                value: description(),
                oninput: move |e: FormEvent| description.set(e.value()),
                onchange: commit_description,
            }
            span { class: "settings-budget-name", "({formatted_number})" }
            select {
                onchange: move |e: FormEvent| {
                    let account_type = match e.value().as_str() {
                        "Billing" => BankAccountType::Billing,
                        "Savings" => BankAccountType::Savings,
                        "Personal" => BankAccountType::Personal,
                        "CreditCard" => BankAccountType::CreditCard,
                        _ => BankAccountType::Checking,
                    };
                    spawn(async move {
                        if let Ok(bv) = modify_bank_account(
                                budget_id,
                                account_id,
                                Some(account_type),
                                None,
                                period_id,
                            )
                            .await
                        {
                            consume_context::<BudgetState>().0.set(bv);
                        }
                    });
                },
                option {
                    value: "Checking",
                    selected: account.account_type == BankAccountType::Checking,
                    "Vanligt konto",
                }
                option {
                    value: "Billing",
                    selected: account.account_type == BankAccountType::Billing,
                    "Räkningskonto",
                }
                option {
                    value: "Savings",
                    selected: account.account_type == BankAccountType::Savings,
                    "Sparkonto",
                }
                option {
                    value: "Personal",
                    selected: account.account_type == BankAccountType::Personal,
                    "Personligt konto",
                }
                option {
                    value: "CreditCard",
                    selected: account.account_type == BankAccountType::CreditCard,
                    "Kreditkort",
                }
            }
        }
    }
}
