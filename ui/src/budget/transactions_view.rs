use crate::budget::{ItemSelector, NewBudgetItem};
use api::models::BudgetingType;
use api::view_models::{AllocationViewModel, BudgetItemViewModel, TransactionViewModel, TransferPair};
use dioxus::prelude::*;
use uuid::Uuid;
use api::connect_transaction;
use crate::{Button, Input, PopoverContent, PopoverRoot, PopoverTrigger};
use crate::budget::budget_hero::BudgetState;

#[component]
pub fn TransactionsView(ignored: bool) -> Element {
    let budget_signal = use_context::<BudgetState>().0;

    let transactions = if ignored {
        budget_signal().ignored_transactions.clone()
    } else {
        budget_signal().to_connect.clone()
    };

    rsx! {
        div { class: "transactions-view-a",
            h2 { class: "transactions-title",
                if ignored {
                    "Ignorerade transaktioner "
                } else {
                    "Ohanterade transaktioner "
                }
                span { class: "transaction-count", "({transactions.len()})" }
            }
            div { class: "transactions-list",
                for tx in transactions {
                    TransactionCard { tx, ignored }
                }
            }
        }
    }
}

#[component]
fn TransactionCard(tx: TransactionViewModel, ignored: bool) -> Element {
    let budget_signal = use_context::<BudgetState>().0;

    rsx! {
        div { class: "transaction-card", key: "{tx.tx_id}",
            div { class: "transaction-info",
                div { class: "transaction-description",
                    strong { {tx.description.to_string()} }
                }
                div { class: "transaction-meta",
                    span { class: "transaction-date",
                        {tx.date.format("%Y-%m-%d").to_string()}
                    }
                    span {
                        class: if tx.amount.is_pos() {
                            "transaction-amount positive"
                        } else {
                            "transaction-amount negative"
                        },
                        {tx.amount.to_string()}
                    }
                }
                if !tx.allocations.is_empty() {
                    div { class: "allocation-list",
                        for alloc in tx.allocations.clone() {
                            AllocationChip { alloc, transaction_id: tx.tx_id }
                        }
                    }
                }
            }
            if !ignored {
                div { class: "transaction-actions",
                    div { class: "action-group",
                        ItemSelector {
                            items: budget_signal().items.clone(),
                            on_change: {
                                let tx = tx.clone();
                                move |e: Option<BudgetItemViewModel>| {
                                    let tx = tx.clone();
                                    async move {
                                        if let Some(item) = e {
                                            info!("Connecting transaction {} to item {}", tx.tx_id, item.item_id);
                                            match connect_transaction(
                                                budget_signal().id,
                                                tx.tx_id,
                                                item.actual_id,
                                                item.item_id,
                                                budget_signal().period_id,
                                            )
                                            .await
                                            {
                                                Ok(bv) => {
                                                    consume_context::<BudgetState>().0.set(bv);
                                                }
                                                Err(e) => {
                                                    error!("Failed to connect transaction: {}", e);
                                                }
                                            }
                                        }
                                    }
                                }
                            },
                        }
                        SplitTransactionPopover { tx: tx.clone() }
                    }
                    div { class: "action-group",
                        if tx.amount.is_pos() {
                            NewBudgetItemPopover {
                                budgeting_type: BudgetingType::Income,
                                tx_id: Some(tx.tx_id),
                            }
                        } else {
                            NewBudgetItemPopover {
                                budgeting_type: BudgetingType::Expense,
                                tx_id: Some(tx.tx_id),
                            }
                            NewBudgetItemPopover {
                                budgeting_type: BudgetingType::Savings,
                                tx_id: Some(tx.tx_id),
                            }
                        }
                    }
                    Button {
                        r#type: "button",
                        "data-style": "destructive",
                        onclick: {
                            let tx_id = tx.tx_id;
                            move |_| async move {
                                info!("Ignoring: {} in {}", tx_id, budget_signal().period_id);
                                if let Ok(bv) = api::ignore_transaction(
                                    budget_signal().id,
                                    tx_id,
                                    budget_signal().period_id,
                                )
                                .await
                                {
                                    consume_context::<BudgetState>().0.set(bv);
                                }
                            }
                        },
                        "Ignorera"
                    }
                }
            }
        }
    }
}

#[component]
fn AllocationChip(alloc: AllocationViewModel, transaction_id: Uuid) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let alloc_id = alloc.allocation_id;

    rsx! {
        span { class: "allocation-chip",
            span { class: "allocation-chip-tag", "{alloc.tag}" }
            span { class: "allocation-chip-amount", "{alloc.amount}" }
            button {
                class: "allocation-chip-remove",
                r#type: "button",
                title: "Ta bort allokering",
                onclick: move |_| async move {
                    if let Ok(bv) = api::delete_allocation(
                        budget_signal().id,
                        alloc_id,
                        transaction_id,
                        budget_signal().period_id,
                    )
                    .await
                    {
                        consume_context::<BudgetState>().0.set(bv);
                    }
                },
                "×"
            }
        }
    }
}

#[component]
fn SplitTransactionPopover(tx: TransactionViewModel) -> Element {
    let mut open = use_signal(|| false);
    let budget_signal = use_context::<BudgetState>().0;
    let mut tag_input = use_signal(String::new);
    let mut amount_input = use_signal(String::new);
    let mut selected_item: Signal<Option<BudgetItemViewModel>> = use_signal(|| None);

    rsx! {
        PopoverRoot {
            open: open(),
            on_open_change: move |v| open.set(v),
            PopoverTrigger { "Dela upp" }
            PopoverContent { gap: "0.5rem",
                div { class: "split-form",
                    p { class: "split-form-title", "Allokera del av transaktion" }
                    p { class: "split-form-total",
                        "Total: " strong { {tx.amount.to_string()} }
                    }
                    div { class: "split-form-field",
                        label { "Tagg" }
                        Input {
                            placeholder: "t.ex. bolåneränta",
                            value: tag_input(),
                            oninput: move |e: FormEvent| tag_input.set(e.value()),
                        }
                    }
                    div { class: "split-form-field",
                        label { "Belopp" }
                        Input {
                            r#type: "number",
                            placeholder: "0",
                            value: amount_input(),
                            oninput: move |e: FormEvent| amount_input.set(e.value()),
                        }
                    }
                    div { class: "split-form-field",
                        label { "Budgetpost" }
                        ItemSelector {
                            items: budget_signal().items.clone(),
                            on_change: move |e: Option<BudgetItemViewModel>| {
                                selected_item.set(e);
                                async move {}
                            },
                        }
                    }
                    div { class: "split-form-actions",
                        Button {
                            r#type: "button",
                            onclick: {
                                let tx = tx.clone();
                                move |_| {
                                    let tx = tx.clone();
                                    let tag = tag_input();
                                    let amount_str = amount_input();
                                    let item = selected_item();
                                    async move {
                                        let Some(item) = item else { return };
                                        let Some(actual_id) = item.actual_id else { return };
                                        let Ok(cents) = amount_str.trim().replace(',', ".").parse::<f64>() else { return };
                                        let amount = api::models::Money::new_cents(
                                            (cents * 100.0) as i64,
                                            budget_signal().currency,
                                        );
                                        if let Ok(bv) = api::create_allocation(
                                            budget_signal().id,
                                            tx.tx_id,
                                            actual_id,
                                            amount,
                                            tag,
                                            budget_signal().period_id,
                                        )
                                        .await
                                        {
                                            consume_context::<BudgetState>().0.set(bv);
                                            tag_input.set(String::new());
                                            amount_input.set(String::new());
                                            selected_item.set(None);
                                            open.set(false);
                                        }
                                    }
                                }
                            },
                            "Lägg till"
                        }
                        Button {
                            r#type: "button",
                            "data-style": "ghost",
                            onclick: move |_| open.set(false),
                            "Avbryt"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn TransferPairsView() -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let pairs = budget_signal().potential_transfers.clone();

    if pairs.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "transactions-view-a",
            h2 { class: "transactions-title",
                "Möjliga interna överföringar "
                span { class: "transaction-count", "({pairs.len()})" }
            }
            div { class: "transactions-list",
                for pair in pairs {
                    TransferPairCard { pair }
                }
            }
        }
    }
}

#[component]
fn TransferPairCard(pair: TransferPair) -> Element {
    let budget_signal = use_context::<BudgetState>().0;
    let out_id = pair.outgoing.tx_id;
    let in_id = pair.incoming.tx_id;

    rsx! {
        div { class: "transaction-card transfer-pair-card", key: "{out_id}-{in_id}",
            div { class: "transfer-pair-row",
                div { class: "transfer-leg",
                    span { class: "transfer-leg-label", "Ut" }
                    span { class: "transaction-description", {pair.outgoing.description.clone()} }
                    span { class: "transaction-date", {pair.outgoing.date.format("%Y-%m-%d").to_string()} }
                    span { class: "transaction-amount negative", {pair.outgoing.amount.to_string()} }
                    span { class: "transfer-account", {pair.outgoing.account_number.clone()} }
                }
                div { class: "transfer-arrow", "⇄" }
                div { class: "transfer-leg",
                    span { class: "transfer-leg-label", "In" }
                    span { class: "transaction-description", {pair.incoming.description.clone()} }
                    span { class: "transaction-date", {pair.incoming.date.format("%Y-%m-%d").to_string()} }
                    span { class: "transaction-amount positive", {pair.incoming.amount.to_string()} }
                    span { class: "transfer-account", {pair.incoming.account_number.clone()} }
                }
            }
            div { class: "transaction-actions",
                Button {
                    r#type: "button",
                    onclick: move |_| async move {
                        let budget_id = budget_signal().id;
                        let period_id = budget_signal().period_id;
                        if let Ok(bv) = api::ignore_transaction(budget_id, out_id, period_id).await {
                            consume_context::<BudgetState>().0.set(bv);
                        }
                        if let Ok(bv) = api::ignore_transaction(budget_id, in_id, period_id).await {
                            consume_context::<BudgetState>().0.set(bv);
                        }
                    },
                    "Bekräfta som intern överföring"
                }
                Button {
                    r#type: "button",
                    "data-style": "ghost",
                    onclick: move |_| async move {
                        if let Ok(bv) = api::ignore_transaction(
                            budget_signal().id,
                            out_id,
                            budget_signal().period_id,
                        ).await {
                            consume_context::<BudgetState>().0.set(bv);
                        }
                    },
                    "Ignorera"
                }
            }
        }
    }
}

#[component]
pub fn NewBudgetItemPopover(budgeting_type: BudgetingType, tx_id: Option<Uuid>) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        PopoverRoot { open: open(), on_open_change: move |v| open.set(v),
            PopoverTrigger { {budgeting_type.to_string()} }
            PopoverContent { gap: "0.25rem",
                NewBudgetItem { budgeting_type, tx_id, close_signal: Some(open) }
            }
        }
    }
}
