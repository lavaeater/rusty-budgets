use api::cqrs::framework::Runtime;
use api::cqrs::runtime::JoyDbBudgetRuntime;
use api::import::import_from_skandia_excel;
use api::models::*;
use chrono::{DateTime, NaiveDate, Utc};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

#[cfg(test)]
#[test]
pub fn create_budget_test() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (res, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.currency, Currency::SEK);

    let res = rt.materialize(budget_id)?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.version, 1);
    assert_eq!(res.currency, Currency::SEK);

    let ser = serde_json::to_string(&res)?;
    let _: Budget = serde_json::from_str(&ser)?;

    Ok(())
}

#[test]
pub fn add_budget_item() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (_, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (res, item_id) = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;

    let item = res.get_item(item_id).unwrap();
    assert_eq!(item.name, "Utgifter");
    assert_eq!(item.budgeting_type, BudgetingType::Expense);

    let budget_agg = rt.materialize(budget_id)?;

    let new_item = budget_agg.get_item(item_id).unwrap();
    assert_eq!(new_item.name, "Utgifter");
    assert_eq!(new_item.budgeting_type, BudgetingType::Expense);
    Ok(())
}

#[test]
pub fn test_trans_hash() {
    let date_str = "2025-10-09";
    let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").unwrap();

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
pub fn connect_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let hundred_money = Money::new_dollars(100, Currency::SEK);

    let (_res, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (_res, item_id) = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::PreviousMonthWorkDayBefore(25));
    let (_res, actual_id) = rt.add_actual(user_id, budget_id, item_id, hundred_money, period_id)?;

    let (_res, tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        &bank_account_number,
        hundred_money,
        hundred_money,
        "Test Transaction",
        now,
    )?;

    let (res, _tx_id) = rt.connect_transaction(user_id, budget_id, tx_id, actual_id)?;

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        hundred_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense, period_id),
        -hundred_money
    );
    Ok(())
}

#[test]
pub fn add_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let (_, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let date_str = "2025-10-26";
    let naive_date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")?;

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
    let mut res = res?.0;
    assert_eq!(res.with_period(period_id).transactions.len(), 1);

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
        "Validation error: Transaction already exists."
    );

    Ok(())
}

#[test]
pub fn test_import_from_skandia_excel() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (_, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (imported, _, _) =
        import_from_skandia_excel(&rt, user_id, budget_id, "./tests/unit-test-data.xlsx")?;
    assert_eq!(imported, 295);
    println!("Imported {} transactions", imported);
    let (omp, not_imported, _) =
        import_from_skandia_excel(&rt, user_id, budget_id, "./tests/unit-test-data.xlsx")?;

    assert_eq!(not_imported, 295);
    assert_eq!(omp, 0);

    let res = rt.load(budget_id)?.unwrap();

    assert_eq!(res.all_transactions().len(), 295);

    Ok(())
}

#[test]
pub fn reconnect_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    let (_res, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (_res, original_item_id) = rt.add_item(
        user_id,
        budget_id,
        "Utgifter".to_string(),
        BudgetingType::Expense,
    )?;

    let (_res, new_item_id) = rt.add_item(
        user_id,
        budget_id,
        "Savings".to_string(),
        BudgetingType::Savings,
    )?;

    let (_res, original_id) = rt.add_actual(
        user_id,
        budget_id,
        original_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;
    let (_res, new_id) = rt.add_actual(
        user_id,
        budget_id,
        new_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;


    let (_res, tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
    )?;

    let (res, _returned_tx_id) =
        rt.connect_transaction(user_id, budget_id, tx_id, original_id)?;

    let expected_money = Money::new_dollars(100, Currency::SEK);

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense, period_id),
        -expected_money
    );
    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Savings, period_id),
        expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Savings, period_id),
        Money::default()
    );

    let (res, _tx_id) = rt.connect_transaction(user_id, budget_id, tx_id, new_id)?;

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense, period_id),
        expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense, period_id),
        Money::default()
    );
    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Savings, period_id),
        expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Savings, period_id),
        -expected_money
    );

    Ok(())
}

#[test]
pub fn reallocate_item_funds() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (_res, budget_id) = rt.create_budget(user_id,"Test Budget", true, MonthBeginsOn::default(), Currency::SEK)?;

    let (_res, from_item_id) = rt.add_item(
        user_id,
        budget_id,
        "Hyra".to_string(),
        BudgetingType::Expense,
    )?;

    let (_res, to_item_id) = rt.add_item(
        user_id,
        budget_id,
        "Livsmedel".to_string(),
        BudgetingType::Expense,
    )?;

    let (_res, from_actual_id) = rt.add_actual(
        user_id,
        budget_id,
        from_item_id,
        Money::new_dollars(100, Currency::SEK),
        period_id,
    )?;
    let (_res, to_actual_id) = rt.add_actual(
        user_id,
        budget_id,
        to_item_id,
        Money::new_dollars(50, Currency::SEK),
        period_id,
    )?;

    let (mut res, _) = rt.reallocate_budgeted_funds(
        user_id,
        budget_id,
        period_id,
        from_actual_id,
        to_actual_id,
        Money::new_dollars(50, Currency::SEK),
    )?;
    let from_item = res.with_period(period_id).get_actual(from_actual_id).unwrap();
    assert_eq!(from_item.budgeted_amount, Money::new_dollars(50, Currency::SEK));
    let to_item = res.with_period(period_id).get_actual(to_actual_id).unwrap();
    assert_eq!(
        to_item.budgeted_amount,
        Money::new_dollars(100, Currency::SEK)
    );
    Ok(())
}

pub fn create_budget_with_items(rt: &JoyDbBudgetRuntime, user_id: Uuid, budget_name: &str, items: Vec<(String, BudgetingType, Money, PeriodId)>) -> anyhow::Result<(Uuid, Vec<(Uuid, Uuid)>)> {
    let (_res, budget_id) = rt.create_budget(user_id, budget_name, true, MonthBeginsOn::default(), Currency::SEK)?;
    let mut item_ids = Vec::new();
    for item in items {
        let (_res, item_id) = rt.add_item(user_id, budget_id, item.0, item.1)?;
        let (_res, actual_id) = rt.add_actual(user_id, budget_id, item_id, item.2, item.3)?;
        item_ids.push((item_id, actual_id));
    }
    Ok((budget_id, item_ids))
}

#[test]
pub fn adjust_item_funds() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let period_id = PeriodId::from_date(Utc::now(), MonthBeginsOn::default());
    
    let (budget_id, items) = create_budget_with_items(&rt, user_id, "Test Budget", vec![("Hyra".to_string(), BudgetingType::Expense, Money::new_dollars(100, Currency::SEK), period_id)])?;
    let (_, actual_id) = items.first().unwrap();
    let (_res, _) = rt.adjust_budgeted_amount(
        user_id,
        budget_id,
        *actual_id,
        period_id,
        Money::new_dollars(-50, Currency::SEK),
    )?;
    
    let mut budget = rt.load(budget_id)?.unwrap();
    
    let item = budget.with_period(period_id).get_actual(*actual_id).unwrap();
    assert_eq!(item.budgeted_amount, Money::new_dollars(50, Currency::SEK));
    Ok(())
}


#[test]
pub fn test_budeting_overview() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let zero_money = Money::new_dollars(0, Currency::SEK);
    let hundred_money = Money::new_dollars(100, Currency::SEK);
    let thousand_money = hundred_money.multiply(10);
    let fivehundred_money = hundred_money.multiply(5);
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());
    
    let (budget_id, items) = create_budget_with_items(&rt, user_id, "Test Budget", vec![
        ("Hyra".to_string(), BudgetingType::Expense, fivehundred_money, period_id),
        ("Lön T".to_string(), BudgetingType::Income, thousand_money, period_id),
        ("Spara".to_string(), BudgetingType::Savings, hundred_money, period_id),
    ])?;


    let budget = rt.load(budget_id)?.unwrap();
    let income_overview = budget
        .get_budgeting_overview(BudgetingType::Income, period_id);
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, zero_money);
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget
        .get_budgeting_overview(BudgetingType::Expense, period_id);
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(expense_overview.actual_amount, zero_money);
    assert_eq!(expense_overview.remaining_budget, fivehundred_money);

    let savings_overview = budget
        .get_budgeting_overview(BudgetingType::Savings, period_id);
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, zero_money);
    assert_eq!(savings_overview.remaining_budget, hundred_money);

    let (_, _) = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[1].1,
        &bank_account_number,
        hundred_money.multiply(9),
        fivehundred_money.multiply(4),
        "Löneinsättning",
        now,
    )?;

    let (_, _) = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[0].1,
        &bank_account_number,
        Money::new_dollars(450, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Bet. Hyra",
        now,
    )?;
    let (_, _) = rt.add_and_connect_tx(
        user_id,
        budget_id,
        items[2].1,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Överföring sparande",
        now,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let income_overview = budget
        .get_budgeting_overview(BudgetingType::Income, period_id);
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, hundred_money.multiply(9));
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget
        .get_budgeting_overview(BudgetingType::Expense, period_id);
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(
        expense_overview.actual_amount,
        Money::new_dollars(-450, Currency::SEK)
    );
    assert_eq!(
        expense_overview.remaining_budget,
        Money::new_dollars(950, Currency::SEK)
    );

    let savings_overview = budget
        .get_budgeting_overview(BudgetingType::Savings, period_id);
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, -hundred_money);
    assert_eq!(savings_overview.remaining_budget, hundred_money.multiply(2));

    Ok(())
}

// ============================================================================
// evaluate_rules tests
// ============================================================================

#[test]
pub fn evaluate_rules_no_rules_returns_empty() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, _) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![("Groceries".to_string(), BudgetingType::Expense, Money::new_dollars(500, Currency::SEK), period_id)],
    )?;

    // Add a transaction
    let (_res, _tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "WILLYS GROCERIES",
        now,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    
    // No rules added, so evaluate_rules should return empty
    let matches = budget.evaluate_rules();
    assert!(matches.is_empty(), "Expected no matches when no rules exist");

    Ok(())
}

#[test]
pub fn evaluate_rules_matches_transaction_to_actual() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![("groceries".to_string(), BudgetingType::Expense, Money::new_dollars(500, Currency::SEK), period_id)],
    )?;
    let (_item_id, actual_id) = items[0];

    // Add a transaction
    let (_res, tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "groceries",
        now,
    )?;

    // Add a rule that matches "groceries" transaction to "groceries" item
    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 1, "Expected one match");
    let (matched_tx_id, matched_actual_id, matched_item_id) = &matches[0];
    assert_eq!(*matched_tx_id, tx_id);
    assert_eq!(*matched_actual_id, Some(actual_id), "Should match to actual");
    assert!(matched_item_id.is_none(), "Item ID should be None when actual is found");

    Ok(())
}

#[test]
pub fn evaluate_rules_matches_transaction_to_item_when_no_actual() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();

    // Create budget with item but NO actual for this period
    let (_res, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    let (_res, item_id) = rt.add_item(
        user_id,
        budget_id,
        "rent".to_string(),
        BudgetingType::Expense,
    )?;

    // Add a transaction
    let (_res, tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-1000, Currency::SEK),
        Money::new_dollars(5000, Currency::SEK),
        "rent payment",
        now,
    )?;

    // Add a rule that matches "rent" transaction to "rent" item
    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["rent".to_string(), "payment".to_string()],
        vec!["rent".to_string()],
        true,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 1, "Expected one match");
    let (matched_tx_id, matched_actual_id, matched_item_id) = &matches[0];
    assert_eq!(*matched_tx_id, tx_id);
    assert!(matched_actual_id.is_none(), "Actual ID should be None when no actual exists");
    assert_eq!(*matched_item_id, Some(item_id), "Should match to item");

    Ok(())
}

#[test]
pub fn evaluate_rules_no_match_for_unrelated_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, _) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![("groceries".to_string(), BudgetingType::Expense, Money::new_dollars(500, Currency::SEK), period_id)],
    )?;

    // Add a transaction with different description
    let (_res, _tx_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-50, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "coffee shop",
        now,
    )?;

    // Add a rule for groceries (won't match "coffee shop")
    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let matches = budget.evaluate_rules();

    assert!(matches.is_empty(), "Expected no matches for unrelated transaction");

    Ok(())
}

#[test]
pub fn evaluate_rules_multiple_transactions_multiple_rules() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let now = Utc::now();
    let period_id = PeriodId::from_date(now, MonthBeginsOn::default());

    let (budget_id, items) = create_budget_with_items(
        &rt,
        user_id,
        "Test Budget",
        vec![
            ("groceries".to_string(), BudgetingType::Expense, Money::new_dollars(500, Currency::SEK), period_id),
            ("utilities".to_string(), BudgetingType::Expense, Money::new_dollars(200, Currency::SEK), period_id),
        ],
    )?;
    let (_groceries_item_id, groceries_actual_id) = items[0];
    let (_utilities_item_id, utilities_actual_id) = items[1];

    // Add transactions
    let (_res, tx1_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-100, Currency::SEK),
        Money::new_dollars(1000, Currency::SEK),
        "groceries",
        now,
    )?;

    let (_res, tx2_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-150, Currency::SEK),
        Money::new_dollars(850, Currency::SEK),
        "utilities",
        now,
    )?;

    let (_res, _tx3_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(-25, Currency::SEK),
        Money::new_dollars(825, Currency::SEK),
        "random purchase",
        now,
    )?;

    // Add rules
    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["groceries".to_string()],
        vec!["groceries".to_string()],
        true,
    )?;

    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["utilities".to_string()],
        vec!["utilities".to_string()],
        true,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 2, "Expected two matches");

    // Check that both transactions are matched to their respective actuals
    let tx1_match = matches.iter().find(|(tx_id, _, _)| *tx_id == tx1_id);
    let tx2_match = matches.iter().find(|(tx_id, _, _)| *tx_id == tx2_id);

    assert!(tx1_match.is_some(), "Transaction 1 should be matched");
    assert!(tx2_match.is_some(), "Transaction 2 should be matched");

    let (_, actual1, _) = tx1_match.unwrap();
    let (_, actual2, _) = tx2_match.unwrap();

    assert_eq!(*actual1, Some(groceries_actual_id));
    assert_eq!(*actual2, Some(utilities_actual_id));

    Ok(())
}

#[test]
pub fn evaluate_rules_across_multiple_periods() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    // Use dates to derive periods
    let date1 = NaiveDate::from_ymd_opt(2025, 10, 15).unwrap()
        .and_hms_opt(0, 0, 0).unwrap().and_utc();
    let date2 = NaiveDate::from_ymd_opt(2025, 11, 15).unwrap()
        .and_hms_opt(0, 0, 0).unwrap().and_utc();

    let period1 = PeriodId::from_date(date1, MonthBeginsOn::default());
    let period2 = PeriodId::from_date(date2, MonthBeginsOn::default());

    let (_res, budget_id) = rt.create_budget(
        user_id,
        "Test Budget",
        true,
        MonthBeginsOn::default(),
        Currency::SEK,
    )?;

    // Create item
    let (_res, item_id) = rt.add_item(
        user_id,
        budget_id,
        "salary".to_string(),
        BudgetingType::Income,
    )?;

    // Add transactions first - this creates the periods
    let (_res, tx1_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(5000, Currency::SEK),
        Money::new_dollars(10000, Currency::SEK),
        "salary",
        date1,
    )?;

    let (_res, tx2_id) = rt.add_transaction(
        user_id,
        budget_id,
        "1234567890",
        Money::new_dollars(5000, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "salary",
        date2,
    )?;

    // Now create actuals in both periods (periods exist now)
    let (_res, actual1_id) = rt.add_actual(
        user_id,
        budget_id,
        item_id,
        Money::new_dollars(5000, Currency::SEK),
        period1,
    )?;

    let (_res, actual2_id) = rt.add_actual(
        user_id,
        budget_id,
        item_id,
        Money::new_dollars(5000, Currency::SEK),
        period2,
    )?;

    // Add rule
    let (_budget, _) = rt.add_rule(
        user_id,
        budget_id,
        vec!["salary".to_string()],
        vec!["salary".to_string()],
        true,
    )?;

    let budget = rt.load(budget_id)?.unwrap();
    let matches = budget.evaluate_rules();

    assert_eq!(matches.len(), 2, "Expected matches from both periods");

    let tx1_match = matches.iter().find(|(tx_id, _, _)| *tx_id == tx1_id);
    let tx2_match = matches.iter().find(|(tx_id, _, _)| *tx_id == tx2_id);

    assert!(tx1_match.is_some());
    assert!(tx2_match.is_some());

    // Each transaction should match to the actual in its respective period
    let (_, actual1, _) = tx1_match.unwrap();
    let (_, actual2, _) = tx2_match.unwrap();

    assert_eq!(*actual1, Some(actual1_id));
    assert_eq!(*actual2, Some(actual2_id));

    Ok(())
}
