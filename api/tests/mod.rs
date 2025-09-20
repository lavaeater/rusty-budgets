use api::cqrs::budget::{BankTransaction, BudgetItemType};
use api::cqrs::framework::Runtime;
use api::cqrs::money::{Currency, Money};
use api::cqrs::runtime::JoyDbBudgetRuntime;
use api::import::import_from_skandia_excel;
use chrono::Utc;
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

#[cfg(test)]
#[test]
pub fn create_budget() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.budget_groups.values().len(), 0);

    let res = rt.materialize(&budget_id)?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.budget_groups.values().len(), 0);
    assert_eq!(res.version, 1);

    Ok(())
}

#[test]
pub fn add_budget_group() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;
    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
    });
    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.budget_groups.values().len(), 1);

    let res = rt.materialize(&budget_id)?;
    assert_eq!(res.name, "Test Budget");
    assert!(res.default_budget);
    assert_eq!(res.budget_groups.values().len(), 1);
    assert_eq!(res.version, 2);
    Ok(())
}

#[test]
pub fn add_budget_group_that_exists() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;
    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
    });
    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.budget_groups.values().len(), 1);
    let res = rt
        .cmd(&user_id, &budget_id, |budget| {
            budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
        })
        .err();
    assert!(res.is_some());
    assert_eq!(
        res.unwrap().to_string(),
        "Validation error: Budget group already exists"
    );

    Ok(())
}

#[test]
pub fn add_budget_item() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;

    let group_id = Uuid::new_v4();
    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(group_id, "Utgifter".to_string())
    })?;
    assert_eq!(res.budget_groups.values().len(), 1);

    let e = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_item(
            group_id,
            "Utgifter".to_string(),
            BudgetItemType::Expense,
            Money::new_dollars(100, Currency::SEK),
        )
    })?;
    let group = e.budget_groups.get(&group_id);
    assert!(group.is_some());
    let group = group.unwrap();
    assert_eq!(group.items.len(), 1);

    let budget_agg = rt.materialize(&budget_id)?;
    println!(
        "Budget {:?}: name={}, default={}",
        budget_agg.id, budget_agg.name, budget_agg.default_budget
    );

    for group in budget_agg.budget_groups.values() {
        println!("Group: {}", group.name);
    }

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
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;

    let now = Utc::now();

    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_transaction(
            Uuid::new_v4(),
            bank_account_number.clone(),
            Money::new_dollars(100, Currency::SEK),
            Money::new_dollars(100, Currency::SEK),
            "Test Transaction".to_string(),
            now,
        )
    });

    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.bank_transactions.len(), 1);

    let res = rt
        .cmd(&user_id, &budget_id, |budget| {
            budget.add_transaction(
                Uuid::new_v4(),
                bank_account_number.clone(),
                Money::new_dollars(100, Currency::SEK),
                Money::new_dollars(100, Currency::SEK),
                "Test Transaction".to_string(),
                now,
            )
        })
        .err();

    assert!(res.is_some());
    assert_eq!(
        res.unwrap().to_string(),
        "Validation error: Transaction already exists."
    );

    Ok(())
}

#[test]
pub fn add_bank_transaction() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let bank_account_number = "1234567890".to_string();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;

    let now = Utc::now();

    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_transaction(
            Uuid::new_v4(),
            bank_account_number.clone(),
            Money::new_dollars(100, Currency::SEK),
            Money::new_dollars(100, Currency::SEK),
            "Test Transaction".to_string(),
            now,
        )
    });

    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.bank_transactions.len(), 1);

    let res = rt
        .cmd(&user_id, &budget_id, |budget| {
            budget.add_transaction(
                Uuid::new_v4(),
                bank_account_number.clone(),
                Money::new_dollars(100, Currency::SEK),
                Money::new_dollars(100, Currency::SEK),
                "Test Transaction".to_string(),
                now,
            )
        })
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
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let _ = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;

    let imported = import_from_skandia_excel("/home/tommie/projects/bealo/rusty-budgets/test_data/91594824853_2025-08-25-2025-09-19.xlsx", &user_id, &budget_id, &rt)?;
    let not_imported =import_from_skandia_excel("/home/tommie/projects/bealo/rusty-budgets/test_data/91594824853_2025-08-25-2025-09-19.xlsx", &user_id, &budget_id, &rt)?;

    let res = rt.load(&budget_id)?.unwrap();

    assert_eq!(res.bank_transactions.len(), 77);
    assert_eq!(imported, 77);
    assert_eq!(not_imported, 0);

    Ok(())
}
