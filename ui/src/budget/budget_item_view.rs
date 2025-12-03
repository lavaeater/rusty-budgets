use crate::budget::ItemSelector;
use crate::Button;
use api::ignore_transaction;
use api::models::{BudgetingType, Money};
use api::view_models::*;
use dioxus::prelude::*;
use lucide_dioxus::Pen;
use std::collections::HashSet;
use uuid::Uuid;

#[component]
pub fn BudgetItemView(item: BudgetItemViewModel) -> Element {
    let mut budget_signal = use_context::<Signal<Option<BudgetViewModel>>>();
    let mut expanded = use_signal(|| false);

    let mut edit_item = use_signal(|| false);
    let item_name = use_signal(|| item.name.clone());
    let mut budgeted_amount = use_signal(|| item.budgeted_amount);

    // State for selected transaction IDs and the target item for moving
    let mut selected_transactions = use_signal(HashSet::<Uuid>::new);
    let mut show_move_selector = use_signal(|| false);
    let budget = budget_signal().unwrap();
    let budget_id = budget.id;

    if expanded() {
        rsx! {
            div { class: "flex flex-col p-2 border-b border-gray-200 text-sm",
                // Header with item name and amount
                div { class: "flex justify-between items-center",
                    // onclick: move |_| { edit_item.set(!edit_item()) },
                    div { class: "font-large", "{item_name()}" }
                    div { class: "text-gray-700",
                        "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                    }
                }
                // Transaction list with checkboxes
                div { class: "mt-2",
                    {
                        item.transactions
                            .iter()
                            .map(|transaction| {
                                let tx_id = transaction.tx_id;
                                let is_selected = selected_transactions().contains(&tx_id);
                                rsx! {
                                    div { class: "flex items-center p-1 hover:bg-gray-50 rounded",
                                        input {
                                            r#type: "checkbox",
                                            checked: is_selected,
                                            onchange: move |_| {
                                                let mut selected = selected_transactions();
                                                if is_selected {
                                                    selected.remove(&tx_id);
                                                } else {
                                                    selected.insert(tx_id);
                                                }
                                                selected_transactions.set(selected);
                                            },
                                        }
                                        {transaction.description.clone()}
                                        {transaction.amount.to_string()}
                                    }
                                }
                            })
                    }
                }
                // Action buttons (only show when transactions are selected)
                if !selected_transactions().is_empty() {
                    div { class: "mt-2 flex items-center gap-2",
                        Button {
                            r#type: "button",
                            "data-style": "secondary",
                            onclick: move |_| {
                                selected_transactions.set(HashSet::new());
                            },
                            "Avmarkera alla"
                        }
                        Button {
                            r#type: "button",
                            "data-style": "destructive",
                            onclick: move |_| async move {
                                let mut success = true;
                                let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                for tx_id in selected_ids {
                                    // Refresh the budget data
                                    if let Err(_) = ignore_transaction(budget_id, tx_id, budget.period_id).await
                                    {
                                        success = false;
                                        break;
                                    }
                                }
                                if success {
                                    if let Ok(updated_budget) = api::get_budget(
                                            Some(budget_id),
                                            budget.period_id,
                                        )
                                        .await
                                    {
                                        budget_signal.set(updated_budget);
                                    }
                                    selected_transactions.set(HashSet::new());
                                    show_move_selector.set(false);
                                }
                            },
                            "Ignorera alla"
                        }

                        if !show_move_selector() {
                            Button {
                                r#type: "button",
                                "data-style": "primary",
                                onclick: move |_| {
                                    show_move_selector.set(true);
                                },
                                "Flytta markerade"
                            }
                        } else {
                            div { class: "flex-1 flex items-center gap-2",
                                span { "Flytta till:" }
                                ItemSelector {
                                    items: budget.items.iter().filter(|i| i.item_id != item.item_id).cloned().collect(),
                                    on_change: move |target_item: Option<BudgetItemViewModel>| async move {
                                        if let Some(target_item) = target_item {
                                            let mut success = true;
                                            let selected_ids: Vec<Uuid> = selected_transactions().into_iter().collect();

                                            for tx_id in selected_ids {
                                                if let Err(_) = api::connect_transaction(
                                                        // Refresh the budget data
                                                        budget_id,
                                                        tx_id,
                                                        target_item.actual_id,
                                                        target_item.item_id,
                                                        budget.period_id,
                                                    )
                                                    .await
                                                {
                                                    success = false;
                                                    break;
                                                }
                                            }
                                            if success {
                                                if let Ok(updated_budget) = api::get_budget(
                                                        Some(budget_id),
                                                        budget.period_id,
                                                    )
                                                    .await
                                                {
                                                    budget_signal.set(updated_budget);
                                                }
                                                selected_transactions.set(HashSet::new());
                                                show_move_selector.set(false);
                                            }
                                        } else {
                                            show_move_selector.set(false);
                                        }
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
        // }
    } else if edit_item() {
        rsx! {
            label { "Edit item" }
            input {
                r#type: "number",
                value: budgeted_amount().to_string(),
                oninput: move |e| {
                    match e.value().parse() {
                        Ok(v) => {
                            budgeted_amount.set(Money::new_dollars(v, budget.currency));
                        }
                        _ => {
                            budgeted_amount.set(Money::zero(budget.currency));
                        }
                    }
                },
            }
            Button {
                onclick: move |_| async move {
                    match api::modify_actual(
                            budget_id,
                            item.actual_id.unwrap(),
                            budget.period_id,
                            Some(budgeted_amount()),
                            None,
                        )
                        // TODO: Show error
                        .await
                    {
                        Ok(updated_budget) => {
                            budget_signal.set(Some(updated_budget));
                            edit_item.set(false)
                        }
                        Err(_) => {
                            edit_item.set(false);
                        }
                    }
                },
                "Save"
            }
        }
    } else {
        rsx! {
            div { class: "budget-item",
                // Left side: name
                div {
                    class: "font-large",
                    onclick: move |_| { expanded.set(!expanded()) },
                    "{item.name}"
                }
                Button { onclick: move |_| { edit_item.set(true) }, Pen {} }
                BudgetItemStatusView { item: item.clone() }
                // Right side: actual / budgeted
                div { class: "text-gray-700",
                    "{item.actual_amount.to_string()} / {item.budgeted_amount.to_string()}"
                }
            }
        }
    }
}

#[component]
pub fn BudgetItemStatusView(item: BudgetItemViewModel) -> Element {
    let mut budget_signal = use_context::<Signal<Option<BudgetViewModel>>>();

    let budget = budget_signal().unwrap();
    let budget_id = budget.id;
    match item.status {
        BudgetItemStatus::Balanced => {
            rsx!()
        }
        BudgetItemStatus::OverBudget => {
            rsx! {
                span { class: "over-budget-indicator", "Over Budget" }
                {
                    let shortage = item.actual_amount - item.budgeted_amount;
                    let can_auto_adjust = item.budgeting_type == BudgetingType::Income
                        || budget
                            .overviews
                            .iter()
                            .find(|o| o.budgeting_type == BudgetingType::Income)
                            .map(|o| o.remaining_budget >= shortage)
                            .unwrap_or(false);
                    rsx! {
                        button {
                            class: "auto-adjust-button",
                            onclick: move |_| async move {
                                let actual_id = item.actual_id.unwrap();
                                let shortage = shortage;

        
                                match api::modify_actual(
                                        budget_id,
                                        actual_id,
                                        budget.period_id,
                                        Some(shortage),
                                        None,
                                    )
                                    .await
                                {
                                    Ok(updated_budget) => {
                                        budget_signal.set(Some(updated_budget));
                                    }
                                    Err(e) => {
                                        error!("Failed to adjust item funds: {}", e);
                                    }
                                }
                            },
                            "Auto-Adjust (+{shortage})"
                        }
                    }
                }
            }
        }
        BudgetItemStatus::NotBudgeted => {
            rsx! {
                span { class: "over-budget-indicator", "Not Budgeted" }
            }
        }
        BudgetItemStatus::UnderBudget => {
            let shortage = item.budgeted_amount - item.actual_amount;
            let can_auto_adjust = true;
            rsx! {
                span { class: "over-budget-indicator", "Under Budget" }
                button {
                    class: "auto-adjust-button",
                    onclick: move |_| async move {
                        let actual_id = item.actual_id.unwrap();
                        let shortage = shortage;

                        match api::modify_actual(
                                budget_id,
                                actual_id,
                                budget.period_id,
                                Some(shortage),
                                None,
                            )
                            .await
                        {
                            Ok(updated_budget) => {
                                budget_signal.set(Some(updated_budget));
                            }
                            Err(e) => {
                                error!("Failed to adjust item funds: {}", e);
                            }
                        }
                    },
                    "Auto-Adjust (+{shortage})"
                }
            }
        }
                
    }
}