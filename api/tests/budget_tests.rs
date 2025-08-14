use api::models::{
    Budget, BudgetCategory, BudgetItem, BudgetTransaction, BudgetTransactionType, MonthBeginsOn,
};
use uuid::Uuid;

fn create_test_user_id() -> Uuid {
    Uuid::new_v4()
}

fn create_test_budget(name: &str, user_id: Uuid) -> Budget {
    Budget::new(name, false, user_id)
}

fn create_income_item(
    budget_id: Uuid,
    name: &str,
    category: &BudgetCategory,
    user_id: Uuid,
    amount: f32,
) -> BudgetItem {
    let mut item = BudgetItem::new(budget_id, name, category, user_id);
    let transaction = BudgetTransaction::new(
        "Initial Budget",
        BudgetTransactionType::StartValue,
        amount,
        None,
        item.id,
        user_id,
    );
    item.store_incoming_transaction(&transaction);
    item
}

fn create_expense_item(
    budget_id: Uuid,
    name: &str,
    category: &BudgetCategory,
    user_id: Uuid,
) -> BudgetItem {
    BudgetItem::new(budget_id, name, category, user_id)
}

#[test]
fn test_budget_new() {
    let user_id = create_test_user_id();
    let budget = Budget::new("Test Budget", true, user_id);

    assert_eq!(budget.name, "Test Budget");
    assert_eq!(budget.default_budget, true);
    assert_eq!(budget.user_id, user_id);
    assert_eq!(budget.month_begins_on, MonthBeginsOn::PreviousMonth(25));
    assert!(budget.budget_items.is_empty());
    assert!(budget.created_at <= chrono::Utc::now().naive_utc());
    assert!(budget.updated_at <= chrono::Utc::now().naive_utc());
}

#[test]
fn test_budget_new_with_default_false() {
    let user_id = create_test_user_id();
    let budget = Budget::new("Non-Default Budget", false, user_id);

    assert_eq!(budget.name, "Non-Default Budget");
    assert_eq!(budget.default_budget, false);
    assert_eq!(budget.user_id, user_id);
}

#[test]
fn test_new_income_category() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let category = budget.new_income_category("Salary");

    assert_eq!(category, BudgetCategory::Income("Salary".to_string()));
    assert!(budget.budget_items.contains_key(&category));
    assert!(budget.budget_items[&category].is_empty());
}

#[test]
fn test_new_expense_category() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let category = budget.new_expense_category("Groceries");

    assert_eq!(category, BudgetCategory::Expense("Groceries".to_string()));
    assert!(budget.budget_items.contains_key(&category));
    assert!(budget.budget_items[&category].is_empty());
}

#[test]
fn test_store_category() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let category = BudgetCategory::Savings("Emergency Fund".to_string());

    budget.store_category(&category);

    assert!(budget.budget_items.contains_key(&category));
    assert!(budget.budget_items[&category].is_empty());
}

#[test]
fn test_store_category_duplicate() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let category = BudgetCategory::Income("Salary".to_string());

    budget.store_category(&category);
    budget.store_category(&category); // Should not create duplicate

    assert_eq!(budget.budget_items.len(), 1);
    assert!(budget.budget_items.contains_key(&category));
}

#[test]
fn test_store_budget_item() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let category = BudgetCategory::Income("Salary".to_string());
    let item = create_income_item(budget.id, "Monthly Salary", &category, user_id, 5000.0);

    budget.store_budget_item(&item);

    assert!(budget.budget_items.contains_key(&category));
    assert_eq!(budget.budget_items[&category].len(), 1);
    assert_eq!(budget.budget_items[&category][0].name, "Monthly Salary");
}

#[test]
fn test_store_multiple_budget_items_same_category() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let category = BudgetCategory::Income("Salary".to_string());

    let item1 = create_income_item(budget.id, "Main Job", &category, user_id, 5000.0);
    let item2 = create_income_item(budget.id, "Side Hustle", &category, user_id, 1000.0);

    budget.store_budget_item(&item1);
    budget.store_budget_item(&item2);

    assert_eq!(budget.budget_items[&category].len(), 2);
    let names: Vec<&String> = budget.budget_items[&category]
        .iter()
        .map(|i| &i.name)
        .collect();
    assert!(names.contains(&&"Main Job".to_string()));
    assert!(names.contains(&&"Side Hustle".to_string()));
}

#[test]
fn test_get_available_spendable_budget_empty() {
    let user_id = create_test_user_id();
    let budget = create_test_budget("Test Budget", user_id);

    let available = budget.get_available_spendable_budget();

    assert_eq!(available, 0.0);
}

#[test]
fn test_get_available_spendable_budget_with_income() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let income_category = BudgetCategory::Income("Salary".to_string());

    let item1 = create_income_item(budget.id, "Main Job", &income_category, user_id, 5000.0);
    let item2 = create_income_item(budget.id, "Side Hustle", &income_category, user_id, 1000.0);

    budget.store_budget_item(&item1);
    budget.store_budget_item(&item2);

    let available = budget.get_available_spendable_budget();

    assert_eq!(available, 6000.0);
}

#[test]
fn test_get_available_spendable_budget_ignores_expenses() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 5000.0);
    let expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item);
    budget.store_budget_item(&expense_item);

    let available = budget.get_available_spendable_budget();

    assert_eq!(available, 5000.0); // Only income counted
}

#[test]
fn test_get_available_spendable_budget_ignores_savings() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let savings_category = BudgetCategory::Savings("Emergency".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 5000.0);
    let savings_item = create_income_item(
        budget.id,
        "Emergency Fund",
        &savings_category,
        user_id,
        1000.0,
    );

    budget.store_budget_item(&income_item);
    budget.store_budget_item(&savings_item);

    let available = budget.get_available_spendable_budget();

    assert_eq!(available, 5000.0); // Only income counted, not savings
}

#[test]
fn test_spend_money_on_sufficient_funds() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 5000.0);
    let mut expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item);
    budget.store_budget_item(&expense_item);

    let initial_income_amount = budget.budget_items[&income_category][0].budgeted_item_amount();
    let initial_expense_amount = expense_item.budgeted_item_amount();

    budget.spend_money_on(&mut expense_item, 1000.0);

    // Check that money was transferred
    assert_eq!(
        budget.budget_items[&income_category][0].budgeted_item_amount(),
        initial_income_amount - 1000.0
    );
    assert_eq!(
        expense_item.budgeted_item_amount(),
        initial_expense_amount + 1000.0
    );

    // Check that transactions were created
    assert_eq!(
        budget.budget_items[&income_category][0]
            .outgoing_transactions
            .len(),
        1
    );
    assert_eq!(expense_item.incoming_transactions.len(), 1);
}

#[test]
fn test_spend_money_on_insufficient_funds() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 1000.0);
    let mut expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item);

    let initial_income_amount = budget.budget_items[&income_category][0].budgeted_item_amount();
    let initial_expense_amount = expense_item.budgeted_item_amount();

    budget.spend_money_on(&mut expense_item, 2000.0); // More than available

    // Check that no money was transferred
    assert_eq!(
        budget.budget_items[&income_category][0].budgeted_item_amount(),
        initial_income_amount
    );
    assert_eq!(expense_item.budgeted_item_amount(), initial_expense_amount);

    // Check that no transactions were created
    assert_eq!(
        budget.budget_items[&income_category][0]
            .outgoing_transactions
            .len(),
        0
    );
    assert_eq!(expense_item.incoming_transactions.len(), 0);
}

#[test]
fn test_spend_money_on_zero_amount() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 1000.0);
    let mut expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item);

    let initial_income_amount = budget.budget_items[&income_category][0].budgeted_item_amount();
    let initial_expense_amount = expense_item.budgeted_item_amount();

    budget.spend_money_on(&mut expense_item, 0.0);

    // Check that no money was transferred
    assert_eq!(
        budget.budget_items[&income_category][0].budgeted_item_amount(),
        initial_income_amount
    );
    assert_eq!(expense_item.budgeted_item_amount(), initial_expense_amount);
}

#[test]
fn test_spend_money_on_negative_amount() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(budget.id, "Salary", &income_category, user_id, 1000.0);
    let mut expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item);

    let initial_income_amount = budget.budget_items[&income_category][0].budgeted_item_amount();
    let initial_expense_amount = expense_item.budgeted_item_amount();

    budget.spend_money_on(&mut expense_item, -500.0);

    // Check that no money was transferred
    assert_eq!(
        budget.budget_items[&income_category][0].budgeted_item_amount(),
        initial_income_amount
    );
    assert_eq!(expense_item.budgeted_item_amount(), initial_expense_amount);
}

#[test]
fn test_spend_money_splits_across_multiple_income_items() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item1 = create_income_item(budget.id, "Main Job", &income_category, user_id, 800.0);
    let income_item2 = create_income_item(budget.id, "Side Job", &income_category, user_id, 500.0);
    let mut expense_item = create_expense_item(budget.id, "Food", &expense_category, user_id);

    budget.store_budget_item(&income_item1);
    budget.store_budget_item(&income_item2);

    budget.spend_money_on(&mut expense_item, 1000.0);

    // Check that money was split correctly
    assert_eq!(
        budget.budget_items[&income_category][0].budgeted_item_amount(),
        0.0
    ); // First item fully spent
    assert_eq!(
        budget.budget_items[&income_category][1].budgeted_item_amount(),
        300.0
    ); // Second item partially spent
    assert_eq!(expense_item.budgeted_item_amount(), 1000.0);

    // Check that transactions were created for both income items
    assert_eq!(
        budget.budget_items[&income_category][0]
            .outgoing_transactions
            .len(),
        1
    );
    assert_eq!(
        budget.budget_items[&income_category][1]
            .outgoing_transactions
            .len(),
        1
    );
    assert_eq!(expense_item.incoming_transactions.len(), 2);
}

#[test]
fn test_budget_partial_equality() {
    let user_id = create_test_user_id();
    let budget1 = Budget::new("Test Budget", true, user_id);
    let mut budget2 = Budget::new("Test Budget", true, user_id);

    // They should not be equal because they have different IDs
    assert_ne!(budget1, budget2);

    // Make them equal by setting the same ID
    budget2.id = budget1.id;
    assert_eq!(budget1, budget2);
}

#[test]
fn test_touch_updates_timestamp() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);
    let original_updated_at = budget.updated_at;

    // Sleep a tiny bit to ensure timestamp difference
    std::thread::sleep(std::time::Duration::from_millis(1));

    budget.touch();

    assert!(budget.updated_at > original_updated_at);
}

#[test]
fn test_budget_display_format() {
    let user_id = create_test_user_id();
    let budget = create_test_budget("My Test Budget", user_id);

    let display_string = format!("{}", budget);

    assert!(display_string.contains("My Test Budget"));
    assert!(display_string.contains(&budget.id.to_string()));
    assert!(display_string.contains("Default Budget: No"));
    assert!(display_string.contains("Month Begins On: Previous month, day 25"));
}

#[test]
fn test_budget_with_items_display() {
    let user_id = create_test_user_id();
    let mut budget = create_test_budget("Test Budget", user_id);

    let income_category = BudgetCategory::Income("Salary".to_string());
    let expense_category = BudgetCategory::Expense("Groceries".to_string());

    let income_item = create_income_item(
        budget.id,
        "Monthly Salary",
        &income_category,
        user_id,
        5000.0,
    );
    let expense_item = create_expense_item(budget.id, "Food Budget", &expense_category, user_id);

    budget.store_budget_item(&income_item);
    budget.store_budget_item(&expense_item);

    let display_string = format!("{}", budget);

    assert!(display_string.contains("Monthly Salary"));
    assert!(display_string.contains("Food Budget"));
    assert!(display_string.contains("Income(Salary)"));
    assert!(display_string.contains("Expense(Groceries)"));
}
