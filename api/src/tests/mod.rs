use crate::cqrs::runtime::{JoyDbBudgetRuntime, StoredBudgetEvent, UserBudgets};
use uuid::Uuid;
use crate::cqrs::budget::{Budget, BudgetItemType};
use crate::cqrs::framework::Runtime;
use crate::cqrs::money::{Currency, Money};

#[cfg(test)]
#[test]
pub fn testy() -> anyhow::Result<()> {
    let mut rt = JoyDbBudgetRuntime::new("data_test.json");
    let budget_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let e = rt.cmd(&user_id, &budget_id, |budget| {
        budget.create_budget("Test Budget".to_string(), user_id, true)
    })?;
    assert_eq!(e.name, "Test Budget");
    assert!(e.default_budget);
    assert_eq!(e.budget_groups.values().len(), 0);
    let res = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
    });
    assert!(res.is_ok());
    let res = res?;
    assert_eq!(res.budget_groups.values().len(), 1);
    let e = rt
        .cmd(&user_id, &budget_id, |budget| {
            budget.add_group(Uuid::new_v4(), "Inkomster".to_string())
        })
        .err();
    assert!(e.is_some());
    assert_eq!(
        e.unwrap().to_string(),
        "Validation error: Budget group already exists"
    );

    let group_id = Uuid::new_v4();
    let e = rt.cmd(&user_id, &budget_id, |budget| {
        budget.add_group(group_id, "Utgifter".to_string())
    })?;
    assert_eq!(e.budget_groups.values().len(), 2);

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
    let _ = rt.db.delete_all_by(|b: &Budget| b.id == budget_id);
    let _ = rt
        .db
        .delete_all_by(|b: &StoredBudgetEvent| b.aggregate_id == budget_id);
    let _ = rt.db.delete_all_by(|b: &UserBudgets| b.id == user_id);
    Ok(())
}
