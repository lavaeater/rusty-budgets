use api::models::*;
use chrono::NaiveDate;
use uuid::Uuid;

/// Helper function to create a test BudgetItem
fn create_test_budget_item() -> BudgetItem {
    let budget_id = Uuid::new_v4();
    let created_by = Uuid::new_v4();
    let category = BudgetCategory::Expense("Test Category".to_string());

    BudgetItem::new(budget_id, "Test Item", &category, created_by)
}

/// Helper function to create a test BudgetItem with Income category
fn create_test_income_budget_item() -> BudgetItem {
    let budget_id = Uuid::new_v4();
    let created_by = Uuid::new_v4();
    let category = BudgetCategory::Income("Test Income".to_string());

    BudgetItem::new(budget_id, "Test Income Item", &category, created_by)
}

/// Helper function to create a test BudgetTransaction
fn create_budget_transaction(
    amount: f32,
    to_item: Uuid,
    from_item: Option<Uuid>,
) -> BudgetTransaction {
    let created_by = Uuid::new_v4();
    BudgetTransaction::new(
        "Test Transaction",
        BudgetTransactionType::default(),
        amount,
        from_item,
        to_item,
        created_by,
    )
}

/// Helper function to create a test BankTransaction
fn create_bank_transaction(amount: f32, budget_item: Uuid) -> BankTransaction {
    let created_by = Uuid::new_v4();
    let bank_date = NaiveDate::from_ymd_opt(2025, 8, 13).unwrap();
    BankTransaction::new_from_user(
        "Test Bank Transaction",
        amount,
        budget_item,
        bank_date,
        created_by,
    )
}

#[test]
fn test_incoming_amount_empty() {
    let budget_item = create_test_budget_item();
    assert_eq!(budget_item.incoming_amount(), 0.0);
}

#[test]
fn test_incoming_amount_single_transaction() {
    let mut budget_item = create_test_budget_item();
    let transaction = create_budget_transaction(100.0, budget_item.id, None);

    budget_item.store_incoming_transaction(&transaction);

    assert_eq!(budget_item.incoming_amount(), 100.0);
}

#[test]
fn test_incoming_amount_multiple_transactions() {
    let mut budget_item = create_test_budget_item();
    let transaction1 = create_budget_transaction(100.0, budget_item.id, None);
    let transaction2 = create_budget_transaction(250.5, budget_item.id, None);
    let transaction3 = create_budget_transaction(75.25, budget_item.id, None);

    budget_item.store_incoming_transaction(&transaction1);
    budget_item.store_incoming_transaction(&transaction2);
    budget_item.store_incoming_transaction(&transaction3);

    assert_eq!(budget_item.incoming_amount(), 425.75);
}

#[test]
fn test_incoming_amount_negative_transactions() {
    let mut budget_item = create_test_budget_item();
    let transaction1 = create_budget_transaction(100.0, budget_item.id, None);
    let transaction2 = create_budget_transaction(-50.0, budget_item.id, None);

    budget_item.store_incoming_transaction(&transaction1);
    budget_item.store_incoming_transaction(&transaction2);

    assert_eq!(budget_item.incoming_amount(), 50.0);
}

#[test]
fn test_incoming_amount_duplicate_transaction_overwrites() {
    let mut budget_item = create_test_budget_item();
    let mut transaction = create_budget_transaction(100.0, budget_item.id, None);

    budget_item.store_incoming_transaction(&transaction);
    assert_eq!(budget_item.incoming_amount(), 100.0);

    // Modify the amount and store again with same ID (should overwrite)
    transaction.amount = 200.0;
    budget_item.store_incoming_transaction(&transaction);

    assert_eq!(budget_item.incoming_amount(), 200.0);
    assert_eq!(budget_item.incoming_transactions.len(), 1);
}

#[test]
fn test_outgoing_amount_empty() {
    let budget_item = create_test_budget_item();
    assert_eq!(budget_item.outgoing_amount(), 0.0);
}

#[test]
fn test_outgoing_amount_single_transaction() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();
    let transaction = create_budget_transaction(150.0, budget_item.id, Some(from_item));

    budget_item.store_outgoing_transaction(&transaction);

    assert_eq!(budget_item.outgoing_amount(), 150.0);
}

#[test]
fn test_outgoing_amount_multiple_transactions() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();
    let transaction1 = create_budget_transaction(50.0, budget_item.id, Some(from_item));
    let transaction2 = create_budget_transaction(125.75, budget_item.id, Some(from_item));
    let transaction3 = create_budget_transaction(24.25, budget_item.id, Some(from_item));

    budget_item.store_outgoing_transaction(&transaction1);
    budget_item.store_outgoing_transaction(&transaction2);
    budget_item.store_outgoing_transaction(&transaction3);

    assert_eq!(budget_item.outgoing_amount(), 200.0);
}

#[test]
fn test_outgoing_amount_negative_transactions() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();
    let transaction1 = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    let transaction2 = create_budget_transaction(-30.0, budget_item.id, Some(from_item));

    budget_item.store_outgoing_transaction(&transaction1);
    budget_item.store_outgoing_transaction(&transaction2);

    assert_eq!(budget_item.outgoing_amount(), 70.0);
}

#[test]
fn test_outgoing_amount_duplicate_transaction_overwrites() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();
    let mut transaction = create_budget_transaction(100.0, budget_item.id, Some(from_item));

    budget_item.store_outgoing_transaction(&transaction);
    assert_eq!(budget_item.outgoing_amount(), 100.0);

    // Modify the amount and store again with same ID (should overwrite)
    transaction.amount = 300.0;
    budget_item.store_outgoing_transaction(&transaction);

    assert_eq!(budget_item.outgoing_amount(), 300.0);
    assert_eq!(budget_item.outgoing_transactions.len(), 1);
}

#[test]
fn test_budgeted_amount_no_transactions() {
    let budget_item = create_test_budget_item();
    assert_eq!(budget_item.budgeted_item_amount(), 0.0);
}

#[test]
fn test_budgeted_amount_only_incoming() {
    let mut budget_item = create_test_budget_item();
    let transaction1 = create_budget_transaction(100.0, budget_item.id, None);
    let transaction2 = create_budget_transaction(50.0, budget_item.id, None);

    budget_item.store_incoming_transaction(&transaction1);
    budget_item.store_incoming_transaction(&transaction2);

    assert_eq!(budget_item.budgeted_item_amount(), 150.0);
}

#[test]
fn test_budgeted_amount_only_outgoing() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();
    let transaction1 = create_budget_transaction(75.0, budget_item.id, Some(from_item));
    let transaction2 = create_budget_transaction(25.0, budget_item.id, Some(from_item));

    budget_item.store_outgoing_transaction(&transaction1);
    budget_item.store_outgoing_transaction(&transaction2);

    assert_eq!(budget_item.budgeted_item_amount(), -100.0);
}

#[test]
fn test_budgeted_amount_incoming_and_outgoing() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();

    // Add incoming transactions
    let incoming1 = create_budget_transaction(500.0, budget_item.id, None);
    let incoming2 = create_budget_transaction(200.0, budget_item.id, None);
    budget_item.store_incoming_transaction(&incoming1);
    budget_item.store_incoming_transaction(&incoming2);

    // Add outgoing transactions
    let outgoing1 = create_budget_transaction(150.0, budget_item.id, Some(from_item));
    let outgoing2 = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_outgoing_transaction(&outgoing1);
    budget_item.store_outgoing_transaction(&outgoing2);

    // 700 incoming - 250 outgoing = 450
    assert_eq!(budget_item.budgeted_item_amount(), 450.0);
}

#[test]
fn test_budgeted_amount_equal_incoming_outgoing() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();

    let incoming = create_budget_transaction(300.0, budget_item.id, None);
    let outgoing = create_budget_transaction(300.0, budget_item.id, Some(from_item));

    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);

    assert_eq!(budget_item.budgeted_item_amount(), 0.0);
}

#[test]
fn test_total_bank_amount_empty() {
    let budget_item = create_test_budget_item();
    assert_eq!(budget_item.total_bank_amount(), 0.0);
}

#[test]
fn test_total_bank_amount_single_transaction() {
    let mut budget_item = create_test_budget_item();
    let bank_transaction = create_bank_transaction(250.0, budget_item.id);

    budget_item.store_bank_transaction(&bank_transaction);

    assert_eq!(budget_item.total_bank_amount(), 250.0);
}

#[test]
fn test_total_bank_amount_multiple_transactions() {
    let mut budget_item = create_test_budget_item();
    let bank_transaction1 = create_bank_transaction(100.0, budget_item.id);
    let bank_transaction2 = create_bank_transaction(75.5, budget_item.id);
    let bank_transaction3 = create_bank_transaction(124.25, budget_item.id);

    budget_item.store_bank_transaction(&bank_transaction1);
    budget_item.store_bank_transaction(&bank_transaction2);
    budget_item.store_bank_transaction(&bank_transaction3);

    assert_eq!(budget_item.total_bank_amount(), 299.75);
}

#[test]
fn test_total_bank_amount_negative_transactions() {
    let mut budget_item = create_test_budget_item();
    let bank_transaction1 = create_bank_transaction(200.0, budget_item.id);
    let bank_transaction2 = create_bank_transaction(-50.0, budget_item.id);
    let bank_transaction3 = create_bank_transaction(-25.0, budget_item.id);

    budget_item.store_bank_transaction(&bank_transaction1);
    budget_item.store_bank_transaction(&bank_transaction2);
    budget_item.store_bank_transaction(&bank_transaction3);

    assert_eq!(budget_item.total_bank_amount(), 125.0);
}

#[test]
fn test_total_bank_amount_duplicate_transaction_overwrites() {
    let mut budget_item = create_test_budget_item();
    let mut bank_transaction = create_bank_transaction(100.0, budget_item.id);

    budget_item.store_bank_transaction(&bank_transaction);
    assert_eq!(budget_item.total_bank_amount(), 100.0);

    // Modify the amount and store again with same ID (should overwrite)
    bank_transaction.amount = 500.0;
    budget_item.store_bank_transaction(&bank_transaction);

    assert_eq!(budget_item.total_bank_amount(), 500.0);
    assert_eq!(budget_item.bank_transactions.len(), 1);
}

#[test]
fn test_comprehensive_scenario() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();

    // Add various incoming transactions (salary, bonus)
    let salary = create_budget_transaction(3000.0, budget_item.id, None);
    let bonus = create_budget_transaction(500.0, budget_item.id, None);
    budget_item.store_incoming_transaction(&salary);
    budget_item.store_incoming_transaction(&bonus);

    // Add outgoing transactions (transfers to expenses)
    let rent_transfer = create_budget_transaction(1200.0, budget_item.id, Some(from_item));
    let food_transfer = create_budget_transaction(400.0, budget_item.id, Some(from_item));
    budget_item.store_outgoing_transaction(&rent_transfer);
    budget_item.store_outgoing_transaction(&food_transfer);

    // Add bank transactions (actual spending)
    let grocery_spend = create_bank_transaction(-150.0, budget_item.id);
    let gas_spend = create_bank_transaction(-75.0, budget_item.id);
    let refund = create_bank_transaction(25.0, budget_item.id);
    budget_item.store_bank_transaction(&grocery_spend);
    budget_item.store_bank_transaction(&gas_spend);
    budget_item.store_bank_transaction(&refund);

    // Verify all amounts
    assert_eq!(budget_item.incoming_amount(), 3500.0); // 3000 + 500
    assert_eq!(budget_item.outgoing_amount(), 1600.0); // 1200 + 400
    assert_eq!(budget_item.budgeted_item_amount(), 1900.0); // 3500 - 1600
    assert_eq!(budget_item.total_bank_amount(), -200.0); // -150 - 75 + 25
}

#[test]
fn test_zero_amount_transactions() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();

    let zero_incoming = create_budget_transaction(0.0, budget_item.id, None);
    let zero_outgoing = create_budget_transaction(0.0, budget_item.id, Some(from_item));
    let zero_bank = create_bank_transaction(0.0, budget_item.id);

    budget_item.store_incoming_transaction(&zero_incoming);
    budget_item.store_outgoing_transaction(&zero_outgoing);
    budget_item.store_bank_transaction(&zero_bank);

    assert_eq!(budget_item.incoming_amount(), 0.0);
    assert_eq!(budget_item.outgoing_amount(), 0.0);
    assert_eq!(budget_item.budgeted_item_amount(), 0.0);
    assert_eq!(budget_item.total_bank_amount(), 0.0);
}

#[test]
fn test_precision_with_decimals() {
    let mut budget_item = create_test_budget_item();
    let from_item = Uuid::new_v4();

    // Test with precise decimal amounts
    let incoming = create_budget_transaction(123.456, budget_item.id, None);
    let outgoing = create_budget_transaction(23.123, budget_item.id, Some(from_item));
    let bank = create_bank_transaction(-45.789, budget_item.id);

    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);
    budget_item.store_bank_transaction(&bank);

    assert!((budget_item.incoming_amount() - 123.456).abs() < f32::EPSILON);
    assert!((budget_item.outgoing_amount() - 23.123).abs() < f32::EPSILON);
    assert!((budget_item.budgeted_item_amount() - 100.333).abs() < 0.001); // Allow small floating point error
    assert!((budget_item.total_bank_amount() - (-45.789)).abs() < f32::EPSILON);
}

#[test]
fn test_is_balanced_no_transactions() {
    let budget_item = create_test_income_budget_item();
    assert!(!budget_item.is_balanced()); // No transactions, so not balanced
}

#[test]
fn test_is_balanced_expense_category_balanced_amounts() {
    let mut budget_item = create_test_budget_item(); // Expense category
    let from_item = Uuid::new_v4();
    let incoming = create_budget_transaction(100.0, budget_item.id, None);
    let outgoing = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);
    assert!(!budget_item.is_balanced()); // Not Income category
}

#[test]
fn test_is_balanced_income_only_incoming() {
    let mut budget_item = create_test_income_budget_item();
    let transaction = create_budget_transaction(100.0, budget_item.id, None);
    budget_item.store_incoming_transaction(&transaction);
    assert!(!budget_item.is_balanced()); // Missing outgoing transactions
}

#[test]
fn test_is_balanced_income_only_outgoing() {
    let mut budget_item = create_test_income_budget_item();
    let from_item = Uuid::new_v4();
    let transaction = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_outgoing_transaction(&transaction);
    assert!(!budget_item.is_balanced()); // Missing incoming transactions
}

#[test]
fn test_is_balanced_income_unequal_amounts() {
    let mut budget_item = create_test_income_budget_item();
    let from_item = Uuid::new_v4();
    let incoming = create_budget_transaction(150.0, budget_item.id, None);
    let outgoing = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);
    assert!(!budget_item.is_balanced()); // Amounts don't balance (150 - 100 = 50, not 0)
}

#[test]
fn test_is_balanced_income_perfect_balance() {
    let mut budget_item = create_test_income_budget_item();
    let from_item = Uuid::new_v4();
    let incoming = create_budget_transaction(100.0, budget_item.id, None);
    let outgoing = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);
    assert!(budget_item.is_balanced()); // Income + both transactions + zero balance = balanced!
}

#[test]
fn test_is_balanced_income_multiple_transactions_balanced() {
    let mut budget_item = create_test_income_budget_item();
    let from_item = Uuid::new_v4();
    
    // Multiple incoming transactions totaling 300
    let incoming1 = create_budget_transaction(200.0, budget_item.id, None);
    let incoming2 = create_budget_transaction(100.0, budget_item.id, None);
    budget_item.store_incoming_transaction(&incoming1);
    budget_item.store_incoming_transaction(&incoming2);
    
    // Multiple outgoing transactions totaling 300
    let outgoing1 = create_budget_transaction(150.0, budget_item.id, Some(from_item));
    let outgoing2 = create_budget_transaction(150.0, budget_item.id, Some(from_item));
    budget_item.store_outgoing_transaction(&outgoing1);
    budget_item.store_outgoing_transaction(&outgoing2);
    
    assert!(budget_item.is_balanced()); // 300 - 300 = 0, perfectly balanced
}

#[test]
fn test_is_balanced_income_with_bank_transactions() {
    let mut budget_item = create_test_income_budget_item();
    let from_item = Uuid::new_v4();
    
    // Balanced budget transactions
    let incoming = create_budget_transaction(100.0, budget_item.id, None);
    let outgoing = create_budget_transaction(100.0, budget_item.id, Some(from_item));
    budget_item.store_incoming_transaction(&incoming);
    budget_item.store_outgoing_transaction(&outgoing);
    
    // Bank transactions don't affect balance calculation
    let bank = create_bank_transaction(-50.0, budget_item.id);
    budget_item.store_bank_transaction(&bank);
    
    assert!(budget_item.is_balanced()); // Bank transactions don't affect is_balanced
}
