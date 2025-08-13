use api::models::budget::{
    BankTransaction, BudgetCategory, BudgetItem, BudgetTransaction, BudgetTransactionType,
};
use chrono::NaiveDate;
use uuid::Uuid;

fn new_budget_item(name: &str, category: BudgetCategory) -> BudgetItem {
    let budget_id = Uuid::new_v4();
    let created_by = Uuid::new_v4();
    BudgetItem::new(budget_id, name, &category, created_by)
}

#[test]
fn amounts_are_zero_for_empty_budget_item() {
    let item = new_budget_item("Empty", BudgetCategory::Expense("Misc".into()));

    assert_eq!(item.incoming_amount(), 0.0);
    assert_eq!(item.outgoing_amount(), 0.0);
    assert_eq!(item.budgeted_amount(), 0.0);
    assert_eq!(item.total_bank_amount(), 0.0);
}

#[test]
fn sums_multiple_transactions_correctly() {
    let mut item = new_budget_item("Groceries", BudgetCategory::Expense("Groceries".into()));

    // Incoming transactions (e.g., allocation from Income)
    let inc1 = BudgetTransaction::new(
        "Initial Allocation",
        BudgetTransactionType::StartValue,
        300.0,
        None,
        item.id,
        Uuid::new_v4(),
    );
    let inc2 = BudgetTransaction::new(
        "Top up",
        BudgetTransactionType::Adjustment,
        50.0,
        None,
        item.id,
        Uuid::new_v4(),
    );

    item.store_incoming_transaction(&inc1);
    item.store_incoming_transaction(&inc2);

    // Outgoing transactions (money moved out of this item)
    let out1 = BudgetTransaction::new(
        "Move to Eating Out",
        BudgetTransactionType::Adjustment,
        40.0,
        Some(item.id),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    let out2 = BudgetTransaction::new(
        "Move to Savings",
        BudgetTransactionType::Adjustment,
        10.0,
        Some(item.id),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );

    item.store_outgoing_transaction(&out1);
    item.store_outgoing_transaction(&out2);

    // Bank transactions (actual spend at stores)
    let bt1 = BankTransaction::new_from_user(
        "Store A",
        25.0,
        item.id,
        NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
        Uuid::new_v4(),
    );
    let bt2 = BankTransaction::new_from_user(
        "Store B",
        15.0,
        item.id,
        NaiveDate::from_ymd_opt(2025, 1, 12).unwrap(),
        Uuid::new_v4(),
    );

    item.store_bank_transaction(&bt1);
    item.store_bank_transaction(&bt2);

    assert!((item.incoming_amount() - 350.0).abs() < f32::EPSILON);
    assert!((item.outgoing_amount() - 50.0).abs() < f32::EPSILON);
    assert!((item.budgeted_amount() - 300.0).abs() < f32::EPSILON);
    assert!((item.total_bank_amount() - 40.0).abs() < f32::EPSILON);
}

#[test]
fn negative_amounts_affect_sums() {
    let mut item = new_budget_item("Refunds", BudgetCategory::Expense("Misc".into()));

    // A negative incoming lowers the incoming sum; still used as-is per implementation
    let inc_neg = BudgetTransaction::new(
        "Correction",
        BudgetTransactionType::Adjustment,
        -20.0,
        None,
        item.id,
        Uuid::new_v4(),
    );
    item.store_incoming_transaction(&inc_neg);

    // A negative outgoing reduces the outgoing sum (i.e., acts like reversing an outgoing)
    let out_neg = BudgetTransaction::new(
        "Reverse Move",
        BudgetTransactionType::Adjustment,
        -5.0,
        Some(item.id),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    item.store_outgoing_transaction(&out_neg);

    // Negative bank transaction (e.g., refund) reduces total bank spend
    let refund = BankTransaction::new_from_user(
        "Refund",
        -12.5,
        item.id,
        NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        Uuid::new_v4(),
    );
    item.store_bank_transaction(&refund);

    assert!((item.incoming_amount() - (-20.0)).abs() < f32::EPSILON);
    assert!((item.outgoing_amount() - (-5.0)).abs() < f32::EPSILON);
    assert!((item.budgeted_amount() - (-15.0)).abs() < f32::EPSILON); // -20 - (-5) = -15
    assert!((item.total_bank_amount() - (-12.5)).abs() < f32::EPSILON);
}
