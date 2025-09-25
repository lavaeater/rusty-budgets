use api::cqrs::budgeting_type::BudgetingType;
use api::cqrs::framework::Runtime;
use api::cqrs::money::{Currency, Money};
use api::cqrs::runtime::JoyDbBudgetRuntime;
use api::import::import_from_skandia_excel;
use chrono::Utc;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;
use api::cqrs::bank_transaction::BankTransaction;

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
        res.budgeted_by_type.get(&BudgetingType::Expense).unwrap(),
        &Money::new_dollars(100, Currency::SEK)
    );

    let budget_agg = rt.materialize(&budget_id)?;
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );
    
    let new_item = budget_agg.get_item(&item_id).unwrap();
    assert_eq!(new_item.name, "Utgifter");
    assert_eq!(new_item.budgeted_amount, Money::new_dollars(100, Currency::SEK));
    
    // audit log
    println!("Events: {:?}", rt.events(&budget_id)?);
    Ok(())
}

#[test]
pub fn test_trans_hash() {
    let now = Utc::now();
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
}

#[test]
pub fn connect_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let (_res, budget_id) = rt.create_budget("Test Budget", true, Currency::SEK, user_id)?;
    
    let (_res, item_id) = rt.add_item(
        &budget_id,
        "Utgifter",
        &BudgetingType::Expense,
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

    let (res, _tx_id) = rt.connect_transaction(budget_id, tx_id, item_id, user_id)?;

    let expected_money = Money::new_dollars(100, Currency::SEK);

    assert_eq!(
        res.budgeted_by_type.get(&BudgetingType::Expense).unwrap(),
        &expected_money
    );
    assert_eq!(
        res.spent_by_type.get(&BudgetingType::Expense).unwrap(),
        &expected_money
    );

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
    assert_eq!(res.bank_transactions.len(), 1);

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
    let not_imported = import_from_skandia_excel(
        "../test_data/91594824853_2025-08-25-2025-09-19.xlsx",
        &user_id,
        &budget_id,
        &rt,
    )?;

    let res = rt.load(&budget_id)?.unwrap();

    assert_eq!(res.bank_transactions.len(), 77);
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
        rt.connect_transaction(budget_id, tx_id, original_item_id, user_id)?;

    let expected_money = Money::new_dollars(100, Currency::SEK);

    assert_eq!(
        res.budgeted_by_type
            .get(&BudgetingType::Expense)
            .expect("Expect the budgeted amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.spent_by_type
            .get(&BudgetingType::Expense)
            .expect("Expect the spent amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.budgeted_by_type
            .get(&BudgetingType::Savings)
            .expect("Expect the budgeted amount for Savings"),
        &expected_money
    );
    assert_eq!(
        res.spent_by_type
            .get(&BudgetingType::Savings)
            .expect("Expect the default amount for Savings"),
        &Money::default()
    );

    let (res, _tx_id) = rt.connect_transaction(budget_id, tx_id, new_item_id, user_id)?;

    assert_eq!(
        res.budgeted_by_type
            .get(&BudgetingType::Expense)
            .expect("Expect the spent amount for Expenses"),
        &expected_money
    );
    assert_eq!(
        res.spent_by_type
            .get(&BudgetingType::Expense)
            .expect("Expect the default spent amount for Expenses"),
        &Money::default()
    );
    assert_eq!(
        res.budgeted_by_type
            .get(&BudgetingType::Savings)
            .expect("Expect the budgeted amount for Savings"),
        &expected_money
    );
    assert_eq!(
        res.spent_by_type
            .get(&BudgetingType::Savings)
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
    assert_eq!(
        item.budgeted_amount,
        Money::new_dollars(50, Currency::SEK)
    );
    Ok(())
}
