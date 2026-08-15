use api::api_error::RustyError;
use api::cqrs::framework::Runtime;
use api::cqrs::runtime::{BudgetCommandsTrait, JoyDbBudgetRuntime};
use api::import::import_from_skandia_excel_sync;
use api::models::*;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

#[cfg(test)]
#[test]
pub fn create_budget_test() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;
    let res = rt.load(budget_id)?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.currency, Currency::SEK);
    assert_eq!(res.version, 1);

    let ser = serde_json::to_string(&res)?;
    let _: Budget = serde_json::from_str(&ser)?;

    Ok(())
}

#[test]
pub fn add_budget_item() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let item_id = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;

    let res = rt.load(budget_id)?;
    let item = res.get_item(item_id).unwrap();
    assert_eq!(item.name, "Utgifter");
    assert_eq!(item.budgeting_type, BudgetingType::Expense);

    let budget_agg = rt.load(budget_id)?;

    let new_item = budget_agg.get_item(item_id).unwrap();
    assert_eq!(new_item.name, "Utgifter");
    assert_eq!(new_item.budgeting_type, BudgetingType::Expense);
    Ok(())
}

#[test]
pub fn test_trans_hash() {
    let date_str = "2025-10-09";
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").unwrap();

    // Convert to midnight UTC
    let now: DateTime<Utc> = naive_date
        .and_hms_opt(0, 0, 0) // hours, minutes, seconds
        .unwrap()
        .and_utc();
    let bank_account_number = "1234567890".to_string();
    let t_a = BankTransaction::new(
        Uuid::new_v4(),
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
    );
    let mut hasher_a = DefaultHasher::new();
    let t_b = BankTransaction::new(
        Uuid::new_v4(),
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
    );
    let mut hasher_b = DefaultHasher::new();
    t_a.hash(&mut hasher_a);
    t_b.hash(&mut hasher_b);
    let hash_a = hasher_a.finish();
    let hash_b = hasher_b.finish();
    assert_eq!(hash_a, hash_b);
    let mut hash_set = HashSet::new();
    hash_set.insert(t_a);
    assert!(!hash_set.insert(t_b));

    let hash_c = get_transaction_hash(
        &Money::new_dollars(100, Currency::SEK),
        &Money::new_dollars(100, Currency::SEK),
        &bank_account_number,
        "Test Transaction",
        &now,
    );
    assert_eq!(hash_a, hash_c);

    let mut set = HashSet::new();
    set.insert(hash_a);
    assert!(!set.insert(hash_b));
    assert!(set.contains(&hash_c));

    let sets: Vec<HashSet<u64>> = vec![HashSet::new(), HashSet::new(), HashSet::new()];

    assert!(sets.iter().all(|s| !s.contains(&hash_a)));

    let sets: Vec<HashSet<u64>> = vec![HashSet::new(), HashSet::new(), set];

    assert!(!sets.iter().all(|s| !s.contains(&hash_a)));
}

#[test]
pub fn connect_bank_transaction() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let hundred_money = Money::new_dollars(100, Currency::SEK);

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let item_id = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::PreviousMonthWorkDayBefore(25));
    let actual_id = rt.add_actual(user_id, budget_id, item_id, hundred_money, period_id)?;

    let tx_id = rt.add_transaction(
        user_id,
        budget_id,
        &bank_account_number,
        hundred_money,
        hundred_money,
        "Test Transaction",
        now,
    )?;

    let _tx_id = rt.connect_transaction(user_id, budget_id, tx_id, actual_id)?;

    let budget = rt.load(budget_id)?;
    assert_eq!(
        budget.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        hundred_money
    );
    assert_eq!(
        budget.get_actual_by_type(&BudgetingType::Expense, period_id),
        -hundred_money
    );
    Ok(())
}

#[test]
pub fn add_bank_transaction() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let date_str = "2025-10-26";
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;

    // Convert to midnight UTC
    let now: DateTime<Utc> = naive_date
        .and_hms_opt(0, 0, 0) // hours, minutes, seconds
        .unwrap()
        .and_utc();

    let period_id = PeriodId::from_date(now, MonthBeginsOn::PreviousMonthWorkDayBefore(25));

    let res = rt.add_transaction(
        user_id,
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
    );

    assert!(res.is_ok());
    let mut budget = rt.load(budget_id)?;
    assert_eq!(budget.with_period(period_id).transactions.len(), 1);

    let also_now: DateTime<Utc> = naive_date
        .and_hms_opt(0, 0, 0) // hours, minutes, seconds
        .unwrap()
        .and_utc();

    let res = rt
        .add_transaction(
            user_id,
            budget_id,
            &bank_account_number,
            Money::new_dollars(100, Currency::SEK),
            Money::new_dollars(100, Currency::SEK),
            "Test Transaction",
            also_now,
        )
        .err();

    assert!(res.is_some());
    assert_eq!(
        res.unwrap().to_string(),
        "Command error: Validation error: Transaction already exists."
    );

    Ok(())
}

#[test]
pub fn test_import_from_skandia_excel() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (imported, _, _) =
        import_from_skandia_excel_sync(&rt, user_id, budget_id, "./tests/unit-test-data.xlsx")
            .unwrap();
    assert_eq!(imported, 296);
    println!("Imported {} transactions", imported);
    let (omp, not_imported, _) =
        import_from_skandia_excel_sync(&rt, user_id, budget_id, "./tests/unit-test-data.xlsx")
            .unwrap();

    assert_eq!(not_imported, 296);
    assert_eq!(omp, 0);

    let budget = rt.load(budget_id)?;
    assert_eq!(budget.all_transactions().len(), 296);

    Ok(())
}

#[test]
pub fn reconnect_bank_transaction() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let original_item_id = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;

    let new_item_id = rt.add_item(
        user_id,
        budget_id,
        "Savings".to_string(),
        BudgetingType::Savings,
    )?;

    let original_id = rt.add_actual(
        user_id,
        budget_id,
        original_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;
    let new_id = rt.add_actual(
        user_id,
        budget_id,
        new_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;

    let tx_id = rt.add_transaction(
        user_id,
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
    )?;

    let _returned_tx_id = rt.connect_transaction(user_id, budget_id, tx_id, original_id)?;

    let expected_money = Money::new_dollars(100, Currency::SEK);

    let budget = rt.load(budget_id)?;
    assert_eq!(
        budget.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        expected_money
    );
    assert_eq!(
        budget.get_actual_by_type(&BudgetingType::Expense, period_id),
        -expected_money
    );
    assert_eq!(
        budget.get_budgeted_by_type(&BudgetingType::Savings, period_id),
        expected_money
    );
    assert_eq!(
        budget.get_actual_by_type(&BudgetingType::Savings, period_id),
        Money::default()
    );

    let _ = rt.connect_transaction(user_id, budget_id, tx_id, new_id)?;

    let budget = rt.load(budget_id)?;
    assert_eq!(
        budget.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        expected_money
    );
    assert_eq!(
        budget.get_actual_by_type(&BudgetingType::Expense, period_id),
        Money::default()
    );
    assert_eq!(
        budget.get_budgeted_by_type(&BudgetingType::Savings, period_id),
        expected_money
    );
    assert_eq!(
        budget.get_actual_by_type(&BudgetingType::Savings, period_id),
        -expected_money
    );

    Ok(())
}

#[test]
pub fn reallocate_item_funds() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let from_item_id = rt.add_item(
        user_id,
        budget_id,
        "Hyra".to_string(),
        BudgetingType::Expense,
    )?;

    let to_item_id = rt.add_item(
        user_id,
        budget_id,
        "Livsmedel".to_string(),
        BudgetingType::Expense,
    )?;

    let from_actual_id = rt.add_actual(
        user_id,
        budget_id,
        from_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;
    let to_actual_id = rt.add_actual(
        user_id,
        budget_id,
        to_item_id,
        Money::new_dollars(50, Currency::SEK),
        period_id,
    )?;

    let _ = rt.reallocate_budgeted_funds(
        user_id,
        budget_id,
        period_id,
        from_actual_id,
        to_actual_id,
        Money::new_dollars(50, Currency::SEK),
    )?;
    let mut budget = rt.load(budget_id)?;
    let from_item = budget
        .with_period(period_id)
        .get_actual(from_actual_id)
        .unwrap();
    assert_eq!(
        from_item.budgeted_amount,
        Money::new_dollars(50, Currency::SEK)
    );
    let to_item = budget
        .with_period(period_id)
        .get_actual(to_actual_id)
        .unwrap();
    assert_eq!(
        to_item.budgeted_amount,
        Money::new_dollars(100, Currency::SEK)
    );
    Ok(())
}

pub fn create_budget_with_items(
    rt: &JoyDbBudgetRuntime,
    user_id: Uuid,
    budget_name: &str,
    items: Vec<(String, BudgetingType, Money, PeriodId)>,
) -> Result<(Uuid, Vec<(Uuid, Uuid)>), RustyError> {
    let budget_id = rt.create_budget(
        user_id,
        budget_name,
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;
    let mut item_ids = Vec::new();
    for item in items {
        let item_id = rt.add_item(user_id, budget_id, item.0, item.1)?;
        let actual_id = rt.add_actual(user_id, budget_id, item_id, item.2, item.3)?;
        item_ids.push((item_id, actual_id));
    }
    Ok((budget_id, item_ids))
}

#[test]
pub fn adjust_item_funds() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let period_id = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![(
            "Hyra".to_string(),
            BudgetingType::Expense,
            Money::new_dollars(100, Currency::SEK),
            period_id,
        )],
    )?;
    let (_, actual_id) = items.first().unwrap();
    let _ = rt.adjust_budgeted_amount(
        user_id,
        budget_id,
        *actual_id,
        period_id,
        Money::new_dollars(-50, Currency::SEK),
    )?;

    let mut budget = rt.load(budget_id)?;

    let item = budget
        .with_period(period_id)
        .get_actual(*actual_id)
        .unwrap();
    assert_eq!(item.budgeted_amount, Money::new_dollars(50, Currency::SEK));
    Ok(())
}

#[test]
pub fn test_budeting_overview() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let zero_money = Money::new_dollars(0, Currency::SEK);
    let hundred_money = Money::new_dollars(100, Currency::SEK);
    let thousand_money = hundred_money.multiply(10);
    let fivehundred_money = hundred_money.multiply(5);
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![
            (
                "Hyra".to_string(),
                BudgetingType::Expense,
                fivehundred_money,
                period_id,
            ),
            (
                "Lön T".to_string(),
                BudgetingType::Income,
                thousand_money,
                period_id,
            ),
            (
                "Spara".to_string(),
                BudgetingType::Savings,
                hundred_money,
                period_id,
            ),
        ],
    )?;

    let budget = rt.load(budget_id)?;
    let income_overview = budget.get_budgeting_overview(BudgetingType::Income, period_id);
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, zero_money);
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget.get_budgeting_overview(BudgetingType::Expense, period_id);
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(expense_overview.actual_amount, zero_money);
    assert_eq!(expense_overview.remaining_budget, fivehundred_money);

    let savings_overview = budget.get_budgeting_overview(BudgetingType::Savings, period_id);
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, zero_money);
    assert_eq!(savings_overview.remaining_budget, hundred_money);

    let _ = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[1].1,
        &bank_account_number,
        hundred_money.multiply(9),
        fivehundred_money.multiply(4),
        "Löneinsättning",
        now,
    )?;

    let _ = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[0].1,
        &bank_account_number,
        Money::new_dollars(450, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Bet. Hyra",
        now,
    )?;
    let _ = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[2].1,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Överföring sparande",
        now,
    )?;

    let budget = rt.load(budget_id)?;
    let income_overview = budget.get_budgeting_overview(BudgetingType::Income, period_id);
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, hundred_money.multiply(9));
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget.get_budgeting_overview(BudgetingType::Expense, period_id);
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(
        expense_overview.actual_amount,
        Money::new_dollars(-450, Currency::SEK)
    );
    assert_eq!(
        expense_overview.remaining_budget,
        Money::new_dollars(950, Currency::SEK)
    );

    let savings_overview = budget.get_budgeting_overview(BudgetingType::Savings, period_id);
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, -hundred_money);
    assert_eq!(savings_overview.remaining_budget, hundred_money.multiply(2));

    Ok(())
}

// ============================================================================
// evaluate_rules tests
// ============================================================================

#[test]
pub fn evaluate_rules_no_rules_returns_empty() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, _) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![(
            "Groceries".to_string(),
            BudgetingType::Expense,
            Money::new_dollars(500, Currency::SEK),
            period_id,
        )],
    )?;

    // Add a transaction
    let _tx_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "WILLYS GROCERIES",
        now,
    )?;

    let budget = rt.load(budget_id)?;

    // No rules added, so evaluate_rules should return empty
    let matches = budget.evaluate_rules();
    assert!(
        matches.is_empty(),
        "Expected no matches when no rules exist"
    );

    Ok(())
}

#[test]
pub fn evaluate_rules_matches_transaction_to_actual() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![(
            "groceries".to_string(),
            BudgetingType::Expense,
            Money::new_dollars(500, Currency::SEK),
            period_id,
        )],
    )?;
    let (_item_id, actual_id) = items[0];

    // Add a transaction
    let tx_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "groceries",
        now,
    )?;

    // Add a rule that matches "groceries" transaction to "groceries" item
    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true, None
        
    )?;

    let budget = rt.load(budget_id)?;
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 1, "Expected one match");
    let m = &matches[0];
    assert_eq!(m.tx_id, tx_id);
    assert_eq!(m.actual_id, Some(actual_id), "Should match to actual");
    assert!(
        m.item_id.is_none(),
        "Item ID should be None when actual is found"
    );

    Ok(())
}

#[test]
pub fn evaluate_rules_matches_transaction_to_item_when_no_actual() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Create budget with item but NO actual for this period
    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let item_id = rt.add_item(
        user_id,
        budget_id,
        "rent".to_string(),
        BudgetingType::Expense,
    )?;

    // Add a transaction
    let tx_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-1000, Currency::SEK),
        Money::new_dollars(5000, Currency::SEK),
        "rent payment",
        now,
    )?;

    // Add a rule that matches "rent" transaction to "rent" item
    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["rent".to_string(), "payment".to_string()],
        vec!["rent".to_string()],
        true,None
    )?;

    let budget = rt.load(budget_id)?;
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 1, "Expected one match");
    let m = &matches[0];
    assert_eq!(m.tx_id, tx_id);
    assert!(
        m.actual_id.is_none(),
        "Actual ID should be None when no actual exists"
    );
    assert_eq!(m.item_id, Some(item_id), "Should match to item");

    Ok(())
}

#[test]
pub fn evaluate_rules_no_match_for_unrelated_transaction() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, _) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![(
            "groceries".to_string(),
            BudgetingType::Expense,
            Money::new_dollars(500, Currency::SEK),
            period_id,
        )],
    )?;

    // Add a transaction with different description
    let _ = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-50, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "coffee shop",
        now,
    )?;

    // Add a rule for groceries (won't match "coffee shop")
    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true, None
    )?;

    let budget = rt.load(budget_id)?;
    let matches = budget.evaluate_rules();

    assert!(
        matches.is_empty(),
        "Expected no matches for unrelated transaction"
    );

    Ok(())
}

#[test]
pub fn evaluate_rules_multiple_transactions_multiple_rules() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![
            (
                "groceries".to_string(),
                BudgetingType::Expense,
                Money::new_dollars(500, Currency::SEK),
                period_id,
            ),
            (
                "utilities".to_string(),
                BudgetingType::Expense,
                Money::new_dollars(200, Currency::SEK),
                period_id,
            ),
        ],
    )?;
    let (_groceries_item_id, groceries_actual_id) = items[0];
    let (_utilities_item_id, utilities_actual_id) = items[1];

    // Add transactions
    let tx1_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "groceries",
        now,
    )?;

    let tx2_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-150, Currency::SEK),
        Money::new_dollars(850, Currency::SEK),
        "utilities",
        now,
    )?;

    let _tx3_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-25, Currency::SEK),
        Money::new_dollars(825, Currency::SEK),
        "random purchase",
        now,
    )?;

    // Add rules
    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true, None
    )?;

    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["utilities".to_string()],
        vec!["utilities".to_string()],
        true, None
    )?;

    let budget = rt.load(budget_id)?;
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 2, "Expected two matches");

    // Check that both transactions are matched to their respective actuals
    let tx1_match = matches.iter().find(|m| m.tx_id == tx1_id);
    let tx2_match = matches.iter().find(|m| m.tx_id == tx2_id);

    assert!(tx1_match.is_some(), "Transaction 1 should be matched");
    assert!(tx2_match.is_some(), "Transaction 2 should be matched");

    assert_eq!(tx1_match.unwrap().actual_id, Some(groceries_actual_id));
    assert_eq!(tx2_match.unwrap().actual_id, Some(utilities_actual_id));

    Ok(())
}

#[test]
pub fn evaluate_rules_across_multiple_periods() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    // Use dates to derive periods
    let date1 = NaiveDate::from_ymd_opt(2025, 10, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
    let date2 = NaiveDate::from_ymd_opt(2025, 11, 15)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

    let period1 = PeriodId::from_date(date1, MonthBeginsOn::default());
    let period2 = PeriodId::from_date(date2, MonthBeginsOn::default());

    let budget_id = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    // Create item
    let item_id = rt.add_item(
        user_id,
        budget_id,
        "salary".to_string(),
        BudgetingType::Income,
    )?;

    // Add transactions first - this creates the periods
    let tx1_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(5000, Currency::SEK),
        Money::new_dollars(10000, Currency::SEK),
        "salary",
        date1,
    )?;

    let tx2_id = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(5000, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "salary",
        date2,
    )?;

    // Now create actuals in both periods (periods exist now)
    let actual1_id = rt.add_actual(
        user_id,
        budget_id,
        item_id,
        Money::new_dollars(5000, Currency::SEK),
        period1,
    )?;

    let actual2_id = rt.add_actual(
        user_id,
        budget_id,
        item_id,
        Money::new_dollars(5000, Currency::SEK),
        period2,
    )?;

    // Add rule
    let _ = rt.add_rule(
        user_id,
        budget_id,
        vec!["salary".to_string()],
        vec!["salary".to_string()],
        true, None
    )?;

    let budget = rt.load(budget_id)?;
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 2, "Expected matches from both periods");

    let tx1_match = matches.iter().find(|m| m.tx_id == tx1_id);
    let tx2_match = matches.iter().find(|m| m.tx_id == tx2_id);

    assert!(tx1_match.is_some());
    assert!(tx2_match.is_some());

    // Each transaction should match to the actual in its respective period
    assert_eq!(tx1_match.unwrap().actual_id, Some(actual1_id));
    assert_eq!(tx2_match.unwrap().actual_id, Some(actual2_id));

    Ok(())
}

// ============================================================================
// tag_created / tag_modified events
// ============================================================================

#[test]
pub fn create_and_modify_tag() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "Tags Budget", true, MonthBeginsOn::default(), Currency::SEK)?;

    let tag_id = rt.create_tag(user_id, budget_id, "Electricity".to_string(), Periodicity::Monthly)?;
    let budget = rt.load(budget_id)?;
    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(tag.name, "Electricity");
    assert_eq!(tag.periodicity(), Some(Periodicity::Monthly));
    assert!(!tag.deleted);

    rt.modify_tag(user_id, budget_id, tag_id, Some("El".to_string()), Some(Periodicity::Annual), None)?;
    let budget = rt.load(budget_id)?;
    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(tag.name, "El");
    assert_eq!(tag.periodicity(), Some(Periodicity::Annual));

    rt.modify_tag(user_id, budget_id, tag_id, None, None, Some(true))?;
    let budget = rt.load(budget_id)?;
    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert!(tag.deleted);

    Ok(())
}

// ============================================================================
// item_modified / item_buffer_set events
// ============================================================================

#[test]
pub fn modify_item_and_set_buffer() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "Buffer Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let item_id = rt.add_item(user_id, budget_id, "Rent".to_string(), BudgetingType::Expense)?;

    rt.modify_item(user_id, budget_id, item_id, Some("Hyra".to_string()), Some(BudgetingType::Savings), None, None)?;
    let budget = rt.load(budget_id)?;
    let item = budget.get_item(item_id).unwrap();
    assert_eq!(item.name, "Hyra");
    assert_eq!(item.budgeting_type, BudgetingType::Savings);

    let buffer = Money::new_dollars(12000, Currency::SEK);
    rt.set_item_buffer(user_id, budget_id, item_id, Some(buffer))?;
    let budget = rt.load(budget_id)?;
    let item = budget.get_item(item_id).unwrap();
    assert_eq!(item.buffer_target, Some(buffer));

    rt.set_item_buffer(user_id, budget_id, item_id, None)?;
    let budget = rt.load(budget_id)?;
    let item = budget.get_item(item_id).unwrap();
    assert!(item.buffer_target.is_none());

    Ok(())
}

// ============================================================================
// rule_modified / rule_deleted events
// ============================================================================

#[test]
pub fn modify_and_delete_rule() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "Rules Budget", true, MonthBeginsOn::default(), Currency::SEK)?;

    let rule_id = rt.add_rule(user_id, budget_id, vec!["willys".to_string()], vec!["groceries".to_string()], true, None)?;

    rt.modify_rule(user_id, budget_id, rule_id, vec!["coop".to_string(), "willys".to_string()])?;
    let budget = rt.load(budget_id)?;
    let rule = budget.match_rules.iter().find(|r| r.id == rule_id).unwrap();
    assert!(rule.transaction_key.contains(&"coop".to_string()));

    rt.delete_rule(user_id, budget_id, rule_id)?;
    let budget = rt.load(budget_id)?;
    assert!(!budget.match_rules.iter().any(|r| r.id == rule_id));

    Ok(())
}

// ============================================================================
// transaction_tagged / transaction_untagged events
// ============================================================================

#[test]
pub fn tag_and_untag_transaction() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let budget_id = rt.create_budget(user_id, "Tag Tx Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tag_id = rt.create_tag(user_id, budget_id, "Groceries".to_string(), Periodicity::Monthly)?;
    let tx_id = rt.add_transaction(user_id, budget_id, "acc123", Money::new_dollars(-50, Currency::SEK), Money::new_dollars(1000, Currency::SEK), "WILLYS", now)?;

    rt.tag_transaction(user_id, budget_id, tx_id, tag_id)?;
    let budget = rt.load(budget_id)?;
    let tx = budget.get_transaction(tx_id).unwrap();
    assert_eq!(tx.tag_id, Some(tag_id));

    rt.untag_transaction(user_id, budget_id, tx_id)?;
    let budget = rt.load(budget_id)?;
    let tx = budget.get_transaction(tx_id).unwrap();
    assert!(tx.tag_id.is_none());

    Ok(())
}

// ============================================================================
// transaction_ignored event
// ============================================================================

#[test]
pub fn ignore_transaction_test() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let budget_id = rt.create_budget(user_id, "Ignore Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tx_id = rt.add_transaction(user_id, budget_id, "acc456", Money::new_dollars(-200, Currency::SEK), Money::new_dollars(5000, Currency::SEK), "Random purchase", now)?;

    rt.ignore_transaction(budget_id, tx_id, user_id)?;
    let budget = rt.load(budget_id)?;
    let tx = budget.get_transaction(tx_id).unwrap();
    assert!(tx.ignored);

    Ok(())
}

// ============================================================================
// transfer_pair_rejected event
// ============================================================================

#[test]
pub fn reject_transfer_pair_test() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let budget_id = rt.create_budget(user_id, "Transfer Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let out_id = rt.add_transaction(user_id, budget_id, "acc1", Money::new_dollars(-1000, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "Transfer out", now)?;
    let in_id = rt.add_transaction(user_id, budget_id, "acc2", Money::new_dollars(1000, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "Transfer in", now)?;

    rt.reject_transfer_pair(user_id, budget_id, out_id, in_id)?;
    let budget = rt.load(budget_id)?;
    assert!(budget.rejected_transfer_pairs.contains(&(out_id, in_id)));

    Ok(())
}

// ============================================================================
// actual_modified event
// ============================================================================

#[test]
pub fn modify_actual_test() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    let budget_id = rt.create_budget(user_id, "Modify Actual Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let item_id = rt.add_item(user_id, budget_id, "Rent".to_string(), BudgetingType::Expense)?;
    let actual_id = rt.add_actual(user_id, budget_id, item_id, Money::new_dollars(500, Currency::SEK), period_id)?;

    rt.modify_actual(user_id, budget_id, actual_id, period_id, Some(Money::new_dollars(600, Currency::SEK)), None)?;
    let mut budget = rt.load(budget_id)?;
    let actual = budget.with_period(period_id).get_actual(actual_id).unwrap();
    assert_eq!(actual.budgeted_amount, Money::new_dollars(600, Currency::SEK));

    Ok(())
}

// ============================================================================
// allocation_created / allocation_deleted events
// ============================================================================

#[test]
pub fn create_and_delete_allocation() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    let budget_id = rt.create_budget(user_id, "Alloc Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let item_id = rt.add_item(user_id, budget_id, "Food".to_string(), BudgetingType::Expense)?;
    let actual_id = rt.add_actual(user_id, budget_id, item_id, Money::new_dollars(300, Currency::SEK), period_id)?;
    let amount = Money::new_dollars(-80, Currency::SEK);
    let tx_id = rt.add_transaction(user_id, budget_id, "acc789", amount, Money::new_dollars(2000, Currency::SEK), "Coop", now)?;

    let alloc_id = rt.create_allocation(user_id, budget_id, tx_id, actual_id, amount, String::new())?;
    let budget = rt.load(budget_id)?;
    assert!(budget.get_period(period_id).unwrap().allocations.iter().any(|a| a.id == alloc_id));

    rt.delete_allocation(user_id, budget_id, alloc_id, tx_id)?;
    let budget = rt.load(budget_id)?;
    assert!(!budget.get_period(period_id).unwrap().allocations.iter().any(|a| a.id == alloc_id));

    Ok(())
}

// ============================================================================
// BudgetViewModel / view model tests
// ============================================================================

#[test]
pub fn budget_view_model_basic() -> Result<(), RustyError> {
    use api::view_models::BudgetViewModel;

    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let budget_id = rt.create_budget(user_id, "VM Budget", true, MonthBeginsOn::default(), Currency::SEK)?;
    let income_id = rt.add_item(user_id, budget_id, "Salary".to_string(), BudgetingType::Income)?;
    let expense_id = rt.add_item(user_id, budget_id, "Rent".to_string(), BudgetingType::Expense)?;
    rt.add_actual(user_id, budget_id, income_id, Money::new_dollars(20000, Currency::SEK), period_id)?;
    rt.add_actual(user_id, budget_id, expense_id, Money::new_dollars(8000, Currency::SEK), period_id)?;
    rt.add_transaction(user_id, budget_id, "acc1", Money::new_dollars(-500, Currency::SEK), Money::new_dollars(10000, Currency::SEK), "Willys mat", now)?;

    let budget = rt.load(budget_id)?;
    let vm = BudgetViewModel::from_budget(&budget, period_id);

    assert_eq!(vm.id, budget_id);
    assert_eq!(vm.name, "VM Budget");
    assert_eq!(vm.currency, Currency::SEK);
    assert_eq!(vm.items.len(), 2);
    assert_eq!(vm.to_connect.len(), 1);
    assert_eq!(vm.untagged_transaction_count, 1);

    let income_ov = vm.overviews.iter().find(|o| o.budgeting_type == BudgetingType::Income).unwrap();
    assert_eq!(income_ov.budgeted_amount, Money::new_dollars(20000, Currency::SEK));

    let expense_ov = vm.overviews.iter().find(|o| o.budgeting_type == BudgetingType::Expense).unwrap();
    assert_eq!(expense_ov.budgeted_amount, Money::new_dollars(8000, Currency::SEK));

    Ok(())
}

#[test]
pub fn transaction_view_model_from_transaction() {
    use api::view_models::TransactionViewModel;

    let account = "acc999".to_string();
    let now = Utc::now();
    let tx = BankTransaction::new(
        Uuid::new_v4(),
        &account,
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(900, Currency::SEK),
        "Test",
        now,
    );
    let vm = TransactionViewModel::from_transaction(&tx);
    assert_eq!(vm.tx_id, tx.id);
    assert_eq!(vm.amount, Money::new_dollars(-100, Currency::SEK));
    assert!(vm.allocations.is_empty());
}

#[test]
pub fn allocation_view_model_from_allocation() {
    use api::models::TransactionAllocation;
    use api::view_models::AllocationViewModel;

    let alloc = TransactionAllocation::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        Money::new_dollars(200, Currency::SEK),
        "food".to_string(),
    );
    let vm = AllocationViewModel::from_allocation(&alloc);
    assert_eq!(vm.allocation_id, alloc.id);
    assert_eq!(vm.amount, Money::new_dollars(200, Currency::SEK));
}

// ============================================================================
// potential_internal_transfers (pure aggregate detection)
// ============================================================================

/// Helper: a UTC datetime at midnight for a given calendar date.
fn at(year: i32, month: u32, day: u32) -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(year, month, day)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

#[test]
pub fn potential_transfers_detects_matching_pair() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    let out_id = rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-1000, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "Överföring ut", at(2025, 6, 10))?;
    let in_id = rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(1000, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "Överföring in", at(2025, 6, 11))?;

    let budget = rt.load(budget_id)?;
    let pairs = budget.potential_internal_transfers();
    assert_eq!(pairs.len(), 1);
    // The pair is (outgoing, incoming) — the negative side is discovered first.
    let (a, b) = pairs[0];
    assert!((a == out_id && b == in_id) || (a == in_id && b == out_id));
    Ok(())
}

#[test]
pub fn potential_transfers_respects_three_day_window() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    // Exactly 3 days apart -> still a pair (boundary is inclusive).
    rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-500, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(500, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "in", at(2025, 6, 13))?;
    let budget = rt.load(budget_id)?;
    assert_eq!(budget.potential_internal_transfers().len(), 1, "3 days apart should pair");

    // 4 days apart -> no pair.
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;
    rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-500, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(500, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "in", at(2025, 6, 14))?;
    let budget = rt.load(budget_id)?;
    assert!(budget.potential_internal_transfers().is_empty(), "4 days apart should not pair");
    Ok(())
}

#[test]
pub fn potential_transfers_requires_different_accounts() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    // Same account, opposite amounts -> not an internal transfer.
    rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-500, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(500, Currency::SEK), Money::new_dollars(9500, Currency::SEK), "in", at(2025, 6, 11))?;
    let budget = rt.load(budget_id)?;
    assert!(budget.potential_internal_transfers().is_empty());
    Ok(())
}

#[test]
pub fn potential_transfers_excludes_rejected_pairs() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    let out_id = rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-750, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    let in_id = rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(750, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "in", at(2025, 6, 11))?;

    // Before rejection: detected.
    assert_eq!(rt.load(budget_id)?.potential_internal_transfers().len(), 1);

    // After rejection: excluded.
    rt.reject_transfer_pair(user_id, budget_id, out_id, in_id)?;
    assert!(rt.load(budget_id)?.potential_internal_transfers().is_empty());
    Ok(())
}

// ============================================================================
// Transfer pair resolution semantics (the CLAUDE.md savings model)
//
// resolve_transfer_pair lives in the async db layer as a composition of
// tag_transaction + ignore_transaction. These tests assert the two resulting
// end-states directly against the runtime:
//   - float:   both sides ignored, neither tagged
//   - savings: outgoing (spending) side tagged, incoming (receipt) side ignored
// ============================================================================

#[test]
pub fn resolve_transfer_pair_float_ignores_both() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    let out_id = rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-1000, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    let in_id = rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(1000, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "in", at(2025, 6, 11))?;

    // Float path: tag_id = None -> ignore both.
    rt.ignore_transaction(budget_id, out_id, user_id)?;
    rt.ignore_transaction(budget_id, in_id, user_id)?;

    let budget = rt.load(budget_id)?;
    let out = budget.get_transaction(out_id).unwrap();
    let inc = budget.get_transaction(in_id).unwrap();
    assert!(out.ignored && inc.ignored, "both sides ignored");
    assert!(out.tag_id.is_none() && inc.tag_id.is_none(), "neither side tagged");
    // Resolved pair no longer surfaces as a potential transfer.
    assert!(budget.potential_internal_transfers().is_empty());
    Ok(())
}

#[test]
pub fn resolve_transfer_pair_savings_tags_outgoing_ignores_incoming() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "T", true, MonthBeginsOn::default(), Currency::SEK)?;

    let savings_tag = rt.create_tag(user_id, budget_id, "Buffert".to_string(), Periodicity::Monthly)?;
    let out_id = rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-1000, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "ut", at(2025, 6, 10))?;
    let in_id = rt.add_transaction(user_id, budget_id, "accB", Money::new_dollars(1000, Currency::SEK), Money::new_dollars(11000, Currency::SEK), "in", at(2025, 6, 11))?;

    // Savings path: tag the outgoing (spending) side, ignore the incoming (receipt).
    rt.tag_transaction(user_id, budget_id, out_id, savings_tag)?;
    rt.ignore_transaction(budget_id, in_id, user_id)?;

    let budget = rt.load(budget_id)?;
    let out = budget.get_transaction(out_id).unwrap();
    let inc = budget.get_transaction(in_id).unwrap();
    assert_eq!(out.tag_id, Some(savings_tag), "outgoing (spending) side carries the savings tag");
    assert!(!out.ignored, "outgoing side is the budget event, not ignored");
    assert!(inc.ignored, "incoming (receipt) side is ignored");
    assert!(inc.tag_id.is_none());
    Ok(())
}

// ============================================================================
// Event-store durability: replay + serde round-trip after many commands.
// ============================================================================

#[test]
pub fn aggregate_survives_serde_round_trip_after_workflow() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    let budget_id = rt.create_budget(user_id, "RT Budget", true, MonthBeginsOn::default(), Currency::SEK)?;

    // Exercise a spread of events: tag, item, actual, transaction, rule.
    let tag_id = rt.create_tag(user_id, budget_id, "Mat".to_string(), Periodicity::Monthly)?;
    let item_id = rt.add_item(user_id, budget_id, "Livsmedel".to_string(), BudgetingType::Expense)?;
    rt.add_actual(user_id, budget_id, item_id, Money::new_dollars(3000, Currency::SEK), period_id)?;
    let tx_id = rt.add_transaction(user_id, budget_id, "accA", Money::new_dollars(-120, Currency::SEK), Money::new_dollars(9000, Currency::SEK), "WILLYS MAT", now)?;
    rt.tag_transaction(user_id, budget_id, tx_id, tag_id)?;
    rt.add_rule(user_id, budget_id, vec!["willys".to_string()], vec![], true, Some(tag_id))?;

    // Load the replayed aggregate, serialise, deserialise, and assert it is stable.
    let budget = rt.load(budget_id)?;
    let json = serde_json::to_string(&budget)?;
    let restored: Budget = serde_json::from_str(&json)?;

    assert_eq!(restored.id, budget.id);
    assert_eq!(restored.version, budget.version);
    assert_eq!(restored.tags.len(), budget.tags.len());
    assert_eq!(restored.items.len(), budget.items.len());
    assert_eq!(restored.match_rules.len(), budget.match_rules.len());
    assert_eq!(restored.get_transaction(tx_id).unwrap().tag_id, Some(tag_id));
    // Re-serialising the restored aggregate must be byte-identical (stable form).
    assert_eq!(serde_json::to_string(&restored)?, json);
    Ok(())
}

// ============================================================================
// tag_classified event + the auto-vs-suggest split.
//
// Bills (Matching::Automatic) are applied on import; card spending
// (Matching::Suggest) is only proposed. Both come from the same rule engine, so
// these assert the two sets are disjoint and that classification is what moves
// a match from one to the other.
// ============================================================================

/// Builds a budget with one tag, one rule (created by tagging a transaction),
/// and a second untagged transaction the rule matches.
fn budget_with_matching_rule(
    rt: &JoyDbBudgetRuntime,
    user_id: Uuid,
    periodicity: Periodicity,
) -> Result<(Uuid, Uuid, Uuid), RustyError> {
    let now = Utc::now();
    let budget_id = rt.create_budget(
        user_id,
        "Rules Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;
    let tag_id = rt.create_tag(user_id, budget_id, "Livsmedel".to_string(), periodicity)?;
    let tagged = rt.add_transaction(
        user_id,
        budget_id,
        "acc123",
        Money::new_dollars(-50, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "WILLYS STORMARKNAD",
        now,
    )?;
    rt.tag_transaction(user_id, budget_id, tagged, tag_id)?;
    rt.add_rule(
        user_id,
        budget_id,
        vec!["willys".to_string(), "stormarknad".to_string()],
        Vec::new(),
        true,
        Some(tag_id),
    )?;
    let untagged = rt.add_transaction(
        user_id,
        budget_id,
        "acc123",
        Money::new_dollars(-70, Currency::SEK),
        Money::new_dollars(930, Currency::SEK),
        "WILLYS STORMARKNAD",
        now,
    )?;
    Ok((budget_id, tag_id, untagged))
}

#[test]
pub fn bills_auto_apply_and_are_not_suggested() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    // A real cadence => Recurring => Matching::Automatic.
    let (budget_id, tag_id, untagged) =
        budget_with_matching_rule(&rt, user_id, Periodicity::Monthly)?;

    let budget = rt.load(budget_id)?;
    assert_eq!(
        budget.evaluate_tag_rules(),
        vec![(untagged, tag_id)],
        "a bill's rule should auto-apply"
    );
    assert!(
        budget.suggest_tag_rules().is_empty(),
        "an automatic match must not also be suggested"
    );
    Ok(())
}

#[test]
pub fn variable_spending_is_suggested_not_auto_applied() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    // OneOff => Variable => Matching::Suggest.
    let (budget_id, tag_id, untagged) =
        budget_with_matching_rule(&rt, user_id, Periodicity::OneOff)?;

    let budget = rt.load(budget_id)?;
    assert!(
        budget.evaluate_tag_rules().is_empty(),
        "card spending must not be tagged without confirmation"
    );
    assert_eq!(
        budget.suggest_tag_rules(),
        vec![(untagged, tag_id)],
        "it should still be offered as a suggestion"
    );
    Ok(())
}

#[test]
pub fn classifying_a_tag_moves_its_matches_between_the_two_sets() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, tag_id, untagged) =
        budget_with_matching_rule(&rt, user_id, Periodicity::OneOff)?;

    // Starts as a suggestion...
    let budget = rt.load(budget_id)?;
    assert!(budget.evaluate_tag_rules().is_empty());
    assert_eq!(budget.suggest_tag_rules(), vec![(untagged, tag_id)]);

    // ...and the user says it is really a monthly bill that matches reliably.
    rt.classify_tag(
        user_id,
        budget_id,
        tag_id,
        CostKind::Recurring(Periodicity::Monthly),
        Matching::Automatic,
    )?;

    let budget = rt.load(budget_id)?;
    assert_eq!(budget.evaluate_tag_rules(), vec![(untagged, tag_id)]);
    assert!(budget.suggest_tag_rules().is_empty());

    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(tag.cost_kind, CostKind::Recurring(Periodicity::Monthly));
    assert!(!tag.needs_review, "classifying answers the review prompt");
    assert!(tag.explicitly_classified);
    Ok(())
}

#[test]
pub fn a_bill_can_be_set_to_suggest_only() -> Result<(), RustyError> {
    // `Bil - Underhåll`: a real recurring cost, but a different garage every
    // time, so the two axes must be settable independently.
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, tag_id, untagged) =
        budget_with_matching_rule(&rt, user_id, Periodicity::Monthly)?;

    rt.classify_tag(
        user_id,
        budget_id,
        tag_id,
        CostKind::Recurring(Periodicity::Annual),
        Matching::Suggest,
    )?;

    let budget = rt.load(budget_id)?;
    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(tag.cost_kind, CostKind::Recurring(Periodicity::Annual));
    assert_eq!(tag.matching, Matching::Suggest);
    assert!(budget.evaluate_tag_rules().is_empty());
    assert_eq!(budget.suggest_tag_rules(), vec![(untagged, tag_id)]);
    Ok(())
}

#[test]
pub fn explicit_classification_survives_a_legacy_periodicity_edit() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "B", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tag_id = rt.create_tag(user_id, budget_id, "Bil".to_string(), Periodicity::OneOff)?;

    rt.classify_tag(
        user_id,
        budget_id,
        tag_id,
        CostKind::Recurring(Periodicity::Annual),
        Matching::Suggest,
    )?;
    // The older periodicity-only path must not clobber a deliberate answer.
    rt.modify_tag(user_id, budget_id, tag_id, None, Some(Periodicity::Monthly), None)?;

    let budget = rt.load(budget_id)?;
    let tag = budget.tags.iter().find(|t| t.id == tag_id).unwrap();
    assert_eq!(tag.cost_kind, CostKind::Recurring(Periodicity::Annual));
    assert_eq!(tag.matching, Matching::Suggest);
    Ok(())
}

#[test]
pub fn deleted_tags_neither_auto_apply_nor_suggest() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, tag_id, _untagged) =
        budget_with_matching_rule(&rt, user_id, Periodicity::Monthly)?;

    rt.modify_tag(user_id, budget_id, tag_id, None, None, Some(true))?;

    let budget = rt.load(budget_id)?;
    assert!(budget.evaluate_tag_rules().is_empty());
    assert!(budget.suggest_tag_rules().is_empty());
    Ok(())
}

// ============================================================================
// Periodisation: a bill charged once a year must budget as 1/12 per month.
// ============================================================================

#[test]
pub fn an_annual_bill_periodises_to_a_twelfth_per_month() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "B", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tag_id = rt.create_tag(user_id, budget_id, "Hund".to_string(), Periodicity::Annual)?;

    let now = Utc::now();
    // One 12 000 kr insurance payment, plus a recent unrelated transaction so
    // the history window is wider than a single day.
    let bill = rt.add_transaction(
        user_id, budget_id, "acc", Money::new_dollars(-12000, Currency::SEK),
        Money::new_dollars(0, Currency::SEK), "HUNDFORSAKRING", now,
    )?;
    rt.tag_transaction(user_id, budget_id, bill, tag_id)?;

    let budget = rt.load(budget_id)?;
    let summary = budget
        .get_tag_summaries()
        .into_iter()
        .find(|s| s.tag_id == tag_id)
        .unwrap();

    assert_eq!(
        summary.monthly_budget_contribution,
        Money::new_dollars(-1000, Currency::SEK),
        "12 000 kr billed yearly should budget as 1 000 kr/month"
    );
    assert_eq!(
        summary.buffer_target(),
        Some(Money::new_dollars(-12000, Currency::SEK)),
        "the buffer must reach the full bill by the time it lands"
    );
    Ok(())
}

#[test]
pub fn variable_spending_is_not_periodised() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "B", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tag_id = rt.create_tag(user_id, budget_id, "Shopping".to_string(), Periodicity::OneOff)?;

    let now = Utc::now();
    let tx = rt.add_transaction(
        user_id, budget_id, "acc", Money::new_dollars(-600, Currency::SEK),
        Money::new_dollars(0, Currency::SEK), "SHOPPING", now,
    )?;
    rt.tag_transaction(user_id, budget_id, tx, tag_id)?;

    let budget = rt.load(budget_id)?;
    let summary = budget
        .get_tag_summaries()
        .into_iter()
        .find(|s| s.tag_id == tag_id)
        .unwrap();

    // Variable costs keep the observed window average and need no buffer.
    assert_eq!(summary.monthly_budget_contribution, summary.average_monthly);
    assert_eq!(summary.buffer_target(), None);
    Ok(())
}

#[test]
pub fn a_monthly_bill_needs_no_buffer() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let budget_id = rt.create_budget(user_id, "B", true, MonthBeginsOn::default(), Currency::SEK)?;
    let tag_id = rt.create_tag(user_id, budget_id, "Bredband".to_string(), Periodicity::Monthly)?;

    let now = Utc::now();
    let tx = rt.add_transaction(
        user_id, budget_id, "acc", Money::new_dollars(-449, Currency::SEK),
        Money::new_dollars(0, Currency::SEK), "BREDBAND", now,
    )?;
    rt.tag_transaction(user_id, budget_id, tx, tag_id)?;

    let budget = rt.load(budget_id)?;
    let summary = budget
        .get_tag_summaries()
        .into_iter()
        .find(|s| s.tag_id == tag_id)
        .unwrap();

    assert_eq!(summary.monthly_budget_contribution, summary.average_monthly);
    assert_eq!(summary.buffer_target(), None, "billed every month, nothing to accumulate");
    Ok(())
}

// ============================================================================
// Confirming suggestions. The inbox confirms via the domain, so these assert
// the state transition rather than the UI: a confirmed suggestion becomes a
// tagged transaction and leaves the suggestion set.
// ============================================================================

/// Two `Suggest` tags with rules, and two untagged transactions each.
fn budget_with_two_suggestion_groups(
    rt: &JoyDbBudgetRuntime,
    user_id: Uuid,
) -> Result<(Uuid, Uuid, Uuid), RustyError> {
    let now = Utc::now();
    let budget_id = rt.create_budget(user_id, "S", true, MonthBeginsOn::default(), Currency::SEK)?;

    let food = rt.create_tag(user_id, budget_id, "Livsmedel".to_string(), Periodicity::OneOff)?;
    let cafe = rt.create_tag(user_id, budget_id, "Café".to_string(), Periodicity::OneOff)?;
    rt.add_rule(user_id, budget_id, vec!["willys".to_string()], Vec::new(), true, Some(food))?;
    rt.add_rule(user_id, budget_id, vec!["espresso".to_string()], Vec::new(), true, Some(cafe))?;

    // Distinct amounts: transactions are deduped by (amount, description, date),
    // so identical rows would be rejected as a duplicate import.
    for (i, desc) in ["WILLYS", "WILLYS", "ESPRESSO HOUSE"].iter().enumerate() {
        rt.add_transaction(
            user_id,
            budget_id,
            "acc",
            Money::new_dollars(-100 - i64::try_from(i).unwrap(), Currency::SEK),
            Money::new_dollars(0, Currency::SEK),
            desc,
            now,
        )?;
    }
    Ok((budget_id, food, cafe))
}

#[test]
pub fn confirming_one_group_leaves_the_other_pending() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, food, cafe) = budget_with_two_suggestion_groups(&rt, user_id)?;

    let budget = rt.load(budget_id)?;
    assert_eq!(budget.suggest_tag_rules().len(), 3, "2 food + 1 cafe");

    // Confirm only the food group, the way the inbox's per-group button does.
    for (tx_id, tag_id) in budget.suggest_tag_rules() {
        if tag_id == food {
            rt.tag_transaction(user_id, budget_id, tx_id, tag_id)?;
        }
    }

    let budget = rt.load(budget_id)?;
    let pending = budget.suggest_tag_rules();
    assert_eq!(pending.len(), 1, "only the cafe suggestion should remain");
    assert_eq!(pending[0].1, cafe);
    assert_eq!(
        budget
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .filter(|t| t.tag_id == Some(food))
            .count(),
        2,
        "both food transactions are now tagged"
    );
    Ok(())
}

#[test]
pub fn confirming_every_suggestion_empties_the_inbox() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, _, _) = budget_with_two_suggestion_groups(&rt, user_id)?;

    let budget = rt.load(budget_id)?;
    for (tx_id, tag_id) in budget.suggest_tag_rules() {
        rt.tag_transaction(user_id, budget_id, tx_id, tag_id)?;
    }

    let budget = rt.load(budget_id)?;
    assert!(budget.suggest_tag_rules().is_empty());
    assert!(
        budget
            .periods
            .iter()
            .flat_map(|p| p.transactions.iter())
            .all(|t| t.tag_id.is_some()),
        "every matched transaction ends up tagged"
    );
    Ok(())
}

#[test]
pub fn a_confirmed_suggestion_does_not_come_back() -> Result<(), RustyError> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, food, _) = budget_with_two_suggestion_groups(&rt, user_id)?;

    let budget = rt.load(budget_id)?;
    let (tx_id, tag_id) = budget
        .suggest_tag_rules()
        .into_iter()
        .find(|(_, t)| *t == food)
        .unwrap();
    rt.tag_transaction(user_id, budget_id, tx_id, tag_id)?;

    let budget = rt.load(budget_id)?;
    assert!(
        !budget.suggest_tag_rules().iter().any(|(t, _)| *t == tx_id),
        "a tagged transaction is no longer suggested"
    );
    Ok(())
}

#[test]
pub fn skipping_is_not_persisted() -> Result<(), RustyError> {
    // The inbox's skip is local-only by design: the transaction stays untagged,
    // so the suggestion must still be there on reload. If this ever starts
    // failing, skip has quietly become a persistent reject.
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let (budget_id, _, _) = budget_with_two_suggestion_groups(&rt, user_id)?;

    let before = rt.load(budget_id)?.suggest_tag_rules().len();
    let after = rt.load(budget_id)?.suggest_tag_rules().len();
    assert_eq!(before, after, "nothing about skipping touches the aggregate");
    assert_eq!(before, 3);
    Ok(())
}
