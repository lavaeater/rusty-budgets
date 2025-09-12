use api::cqrs::budget::BudgetItemType;
use api::cqrs::framework::Runtime;
use api::cqrs::money::{Currency, Money};
use api::cqrs::runtime::JoyDbBudgetRuntime;
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
    Ok(())
}

#[test]
pub fn add_budget_group() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;
    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
    });
    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.budget_groups.values().len(), 1);
    Ok(())
}

#[test]
pub fn add_budget_group_that_exists() -> anyhow::Result<()> {
    let rt = JoyDbBudgetRuntime::new_in_memory();
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let res = rt.cmd(&user_id, &budget_id, |budget| {
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

    let res = rt.cmd(&user_id, &budget_id, |budget| {
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
            Money::new(100, Currency::SEK),
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
