use api::cqrs::framework::Runtime;
use api::cqrs::runtime::JoyDbBudgetRuntime;
use api::import::import_from_skandia_excel;
use api::models::*;
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

#[cfg(test)]
#[test]
pub fn create_budget_test() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.currency, Currency::SEK);

    let res = rt.materialize(&budget_id)?;
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

    let (_, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (res, item_id) = rt.add_item(
        &budget_id,
        "Utgifter",
        &BudgetingType::Expense,
        &Money::new_dollars(100, Currency::SEK),
        &user_id,
    )?;

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense).unwrap(),
        &Money::new_dollars(100, Currency::SEK)
    );

    let budget_agg = rt.materialize(&budget_id)?;
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );

    let new_item = budget_agg.get_item(&item_id).unwrap();
    assert_eq!(new_item.name, "Utgifter");
    assert_eq!(
        new_item.budgeted_amount,
        Money::new_dollars(100, Currency::SEK)
    );

    //Verify that the budget overview is updated
    let income_overview = budget_agg
        .get_budgeting_overview(&BudgetingType::Income)
        .unwrap();
    assert_eq!(
        income_overview.budgeted_amount,
        Money::new_dollars(0, Currency::SEK)
    );
    let expense_overview = budget_agg
        .get_budgeting_overview(&BudgetingType::Expense)
        .unwrap();
    assert_eq!(
        expense_overview.budgeted_amount,
        Money::new_dollars(100, Currency::SEK)
    );
    let savings_overview = budget_agg
        .get_budgeting_overview(&BudgetingType::Savings)
        .unwrap();
    assert_eq!(
        savings_overview.budgeted_amount,
        Money::new_dollars(0, Currency::SEK)
    );

    // audit log
    println!("Events: {:?}", rt.events(&budget_id)?);
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
    
    let sets : Vec<HashSet<u64>>= vec![HashSet::new(), HashSet::new(), HashSet::new()];
    
    assert!(sets.iter().all(|s| !s.contains(&hash_a)));
    
    let sets : Vec<HashSet<u64>>= vec![HashSet::new(), HashSet::new(), set];

    assert!(!sets.iter().all(|s| !s.contains(&hash_a)));
    
}

#[test]
pub fn connect_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();
    let hundred_money = Money::new_dollars(100, Currency::SEK);
    let zero_money = Money::new_dollars(0, Currency::SEK);

    let (_res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (_res, item_id) = rt.add_item(
        &budget_id,
        "Utgifter",
        &BudgetingType::Expense,
        &hundred_money,
        &user_id,
    )?;

    let now = Utc::now();

    let (_res, tx_id) = rt.add_transaction(
        budget_id,
        &bank_account_number,
        hundred_money,
        hundred_money,
        "Test Transaction",
        now,
        user_id,
    )?;

    let (res, _tx_id) = rt.connect_transaction(&budget_id, &tx_id, &item_id, &user_id)?;

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense).unwrap(),
        &hundred_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense).unwrap(),
        &hundred_money
    );

    //Verify that the budget overview is updated
    let income_overview = res.get_budgeting_overview(&BudgetingType::Income).unwrap();
    assert_eq!(income_overview.budgeted_amount, zero_money);
    let expense_overview = res.get_budgeting_overview(&BudgetingType::Expense).unwrap();
    assert_eq!(expense_overview.budgeted_amount, hundred_money);
    let savings_overview = res.get_budgeting_overview(&BudgetingType::Savings).unwrap();
    assert_eq!(savings_overview.budgeted_amount, zero_money);

    Ok(())
}

#[test]
pub fn add_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let (_, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let now = Utc::now();

    let res = rt.add_transaction(
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
        user_id,
    );

    assert!(res.is_ok());
    let res = res?.0;
    assert_eq!(res.list_bank_transactions().len(), 1);

    let res = rt
        .add_transaction(
            budget_id,
            &bank_account_number,
            Money::new_dollars(100, Currency::SEK),
            Money::new_dollars(100, Currency::SEK),
            "Test Transaction",
            now,
            user_id,
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

    let (_, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let imported = import_from_skandia_excel(
        "../test_data/91594824853_2025-08-25-2025-09-19.xlsx",
        &user_id,
        &budget_id,
        &rt,
    )?;

    println!("Imported {} transactions", imported);
    let not_imported = import_from_skandia_excel(
        "../test_data/91594824853_2025-08-25-2025-09-19.xlsx",
        &user_id,
        &budget_id,
        &rt,
    )?;

    println!("Not imported {} transactions", not_imported);

    let mut res = rt.load(&budget_id)?.unwrap();

    let date = Utc::now()
        .with_year(2025)
        .unwrap()
        .with_month(9)
        .unwrap()
        .with_day(19)
        .unwrap();

    res.set_current_period(&date);

    assert_eq!(res.list_all_bank_transactions().len(), 77);
    assert_eq!(imported, 77);
    assert_eq!(not_imported, 0);

    Ok(())
}

#[test]
pub fn reconnect_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let (_res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (_res, original_item_id) = rt.add_item(
        &budget_id,
        "Utgifter",
        &BudgetingType::Expense,
        &Money::new_dollars(100, Currency::SEK),
        &user_id,
    )?;

    let (_res, new_item_id) = rt.add_item(
        &budget_id,
        "Savings",
        &BudgetingType::Savings,
        &Money::new_dollars(100, Currency::SEK),
        &user_id,
    )?;

    let now = Utc::now();

    let (_res, tx_id) = rt.add_transaction(
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(100, Currency::SEK),
        "Test Transaction",
        now,
        user_id,
    )?;

    let (res, _returned_tx_id) =
        rt.connect_transaction(&budget_id, &tx_id, &original_item_id, &user_id)?;

    let expected_money = Money::new_dollars(100, Currency::SEK);

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense)
            .expect("Expect the budgeted amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense)
            .expect("Expect the spent amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Savings)
            .expect("Expect the budgeted amount for Savings"),
        &expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Savings)
            .expect("Expect the default amount for Savings"),
        &Money::default()
    );

    let (res, _tx_id) = rt.connect_transaction(&budget_id, &tx_id, &new_item_id, &user_id)?;

    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Expense)
            .expect("Expect the spent amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Expense)
            .expect("Expect the default spent amount for Expenses"),
        &Money::default()
    );
    assert_eq!(
        res.get_budgeted_by_type(&BudgetingType::Savings)
            .expect("Expect the budgeted amount for Savings"),
        &expected_money
    );
    assert_eq!(
        res.get_actual_by_type(&BudgetingType::Savings)
            .expect("Expect the correct spent amount for Savings"),
        &expected_money
    );

    Ok(())
}

#[test]
pub fn reallocate_item_funds() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (_res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (_res, from_item_id) = rt.add_item(
        &budget_id,
        "Hyra",
        &BudgetingType::Expense,
        &Money::new_dollars(100, Currency::SEK),
        &user_id,
    )?;

    let (_res, to_item_id) = rt.add_item(
        &budget_id,
        "Livsmedel",
        &BudgetingType::Expense,
        &Money::new_dollars(50, Currency::SEK),
        &user_id,
    )?;

    let (res, _) = rt.reallocate_item_funds(
        budget_id,
        from_item_id,
        to_item_id,
        Money::new_dollars(50, Currency::SEK),
        user_id,
    )?;
    let from_item = res.get_item(&from_item_id).unwrap();
    let to_item = res.get_item(&to_item_id).unwrap();
    assert_eq!(
        from_item.budgeted_amount,
        Money::new_dollars(50, Currency::SEK)
    );
    assert_eq!(
        to_item.budgeted_amount,
        Money::new_dollars(100, Currency::SEK)
    );

    Ok(())
}

#[test]
pub fn adjust_item_funds() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();

    let (_res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (_res, item_id) = rt.add_item(
        &budget_id,
        "Hyra",
        &BudgetingType::Expense,
        &Money::new_dollars(100, Currency::SEK),
        &user_id,
    )?;

    let (res, _) = rt.adjust_item_funds(
        budget_id,
        item_id,
        Money::new_dollars(-50, Currency::SEK),
        user_id,
    )?;
    let item = res.get_item(&item_id).unwrap();
    assert_eq!(item.budgeted_amount, Money::new_dollars(50, Currency::SEK));
    Ok(())
}

#[test]
fn test_calculate_rules() {
    use BudgetingType::*;
    use Rule::*;
    let mut store = BudgetItemStore::default();
    store.insert(
        &BudgetItem::new(
            Uuid::new_v4(),
            "Lön",
            Money::new_dollars(5000, Currency::SEK),
            None,
            None,
        ),
        Income,
    );
    store.insert(
        &BudgetItem::new(
            Uuid::new_v4(),
            "Lön",
            Money::new_dollars(3000, Currency::SEK),
            None,
            None,
        ),
        Expense,
    );
    store.insert(
        &BudgetItem::new(
            Uuid::new_v4(),
            "Lön",
            Money::new_dollars(1000, Currency::SEK),
            None,
            None,
        ),
        Savings,
    );

    let income_rule = Sum(vec![Income]);
    let remaining_rule = Difference(Income, vec![Expense, Savings]);

    assert_eq!(
        income_rule.evaluate(&store.hash_by_type(), Some(ValueKind::Budgeted)),
        Money::new_dollars(5000, Currency::SEK)
    );
    assert_eq!(
        remaining_rule.evaluate(&store.hash_by_type(), Some(ValueKind::Budgeted)),
        Money::new_dollars(1000, Currency::SEK)
    );
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

    let (_, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;

    let (_, income_id) = rt.add_item(
        &budget_id,
        "Lön T",
        &BudgetingType::Income,
        &thousand_money,
        &user_id,
    )?;

    let (_, rent_id) = rt.add_item(
        &budget_id,
        "Hyra",
        &BudgetingType::Expense,
        &fivehundred_money,
        &user_id,
    )?;

    let (_, savings_id) = rt.add_item(
        &budget_id,
        "Spara",
        &BudgetingType::Savings,
        &hundred_money,
        &user_id,
    )?;

    let budget = rt.materialize(&budget_id)?;
    let income_overview = budget
        .get_budgeting_overview(&BudgetingType::Income)
        .unwrap();
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, zero_money);
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget
        .get_budgeting_overview(&BudgetingType::Expense)
        .unwrap();
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(expense_overview.actual_amount, zero_money);
    assert_eq!(expense_overview.remaining_budget, fivehundred_money);

    let savings_overview = budget
        .get_budgeting_overview(&BudgetingType::Savings)
        .unwrap();
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, zero_money);
    assert_eq!(savings_overview.remaining_budget, hundred_money);

    let (_, _) = rt.add_and_connect_tx(
        budget_id,
        &bank_account_number,
        hundred_money.multiply(9),
        fivehundred_money.multiply(4),
        "Löneinsättning",
        Utc::now(),
        income_id,
        user_id,
    )?;

    let (_, _) = rt.add_and_connect_tx(
        budget_id,
        &bank_account_number,
        Money::new_dollars(450, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Bet. Hyra",
        Utc::now(),
        rent_id,
        user_id,
    )?;
    let (_, _) = rt.add_and_connect_tx(
        budget_id,
        &bank_account_number,
        Money::new_dollars(100, Currency::SEK),
        Money::new_dollars(15000, Currency::SEK),
        "Överföring sparande",
        Utc::now(),
        savings_id,
        user_id,
    )?;

    let budget = rt.materialize(&budget_id)?;
    let income_overview = budget
        .get_budgeting_overview(&BudgetingType::Income)
        .unwrap();
    assert_eq!(income_overview.budgeted_amount, thousand_money);
    assert_eq!(income_overview.actual_amount, hundred_money.multiply(9));
    assert_eq!(
        income_overview.remaining_budget,
        fivehundred_money - hundred_money
    );

    let expense_overview = budget
        .get_budgeting_overview(&BudgetingType::Expense)
        .unwrap();
    assert_eq!(expense_overview.budgeted_amount, fivehundred_money);
    assert_eq!(
        expense_overview.actual_amount,
        Money::new_dollars(450, Currency::SEK)
    );
    assert_eq!(
        expense_overview.remaining_budget,
        Money::new_dollars(50, Currency::SEK)
    );

    let savings_overview = budget
        .get_budgeting_overview(&BudgetingType::Savings)
        .unwrap();
    assert_eq!(savings_overview.budgeted_amount, hundred_money);
    assert_eq!(savings_overview.actual_amount, hundred_money);
    assert_eq!(savings_overview.remaining_budget, zero_money);

    Ok(())
}
