use crate::models::{
    ActualItem, BudgetItem, Currency, Money,
    PeriodId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
mod budget_item_view_model;
mod budget_item_status;
mod transaction_view_model;
mod budget_view_model;
mod budgeting_type_overview;
pub mod rule;
pub mod value_kind;

pub use budgeting_type_overview::BudgetingTypeOverview;
pub use rule::Rule;
pub use value_kind::ValueKind;
pub use budget_item_view_model::BudgetItemViewModel;
pub use transaction_view_model::TransactionViewModel;
pub use budget_view_model::BudgetViewModel;
pub use budget_item_status::BudgetItemStatus;

#[cfg(test)]
#[test]
fn test_calculate_rules() {
    use crate::models::BudgetingType::*;
    use std::sync::{Arc, Mutex};
    use rule::Rule::*;
    use value_kind::ValueKind;
    let period_id = PeriodId::new(2025, 12);
    let budget_items = [
        BudgetItem::new(Uuid::new_v4(), "Lön", Income),
        BudgetItem::new(Uuid::new_v4(), "Hyra", Expense),
        BudgetItem::new(Uuid::new_v4(), "Spara", Savings),
    ];

    let store = vec![
        ActualItem::new(
            Uuid::new_v4(),
            &budget_items[0].name,
            budget_items[0].id,
            budget_items[0].budgeting_type,
            period_id,
            Money::new_dollars(5000, Currency::SEK),
            Money::new_dollars(4000, Currency::SEK),
            None,
            vec![],
        ),
        ActualItem::new(
            Uuid::new_v4(),
            &budget_items[1].name,
            budget_items[1].id,
            budget_items[1].budgeting_type,
            period_id,
            Money::new_dollars(3000, Currency::SEK),
            Money::new_dollars(2000, Currency::SEK),
            None,
            vec![],
        ),
        ActualItem::new(
            Uuid::new_v4(),
            &budget_items[2].name,
            budget_items[2].id,
            budget_items[2].budgeting_type,
            period_id,
            Money::new_dollars(1000, Currency::SEK),
            Money::new_dollars(500, Currency::SEK),
            None,
            vec![],
        ),
    ];

    let income_rule = Sum(vec![Income]);
    let remaining_rule = Difference(Income, vec![Expense, Savings]);

    assert_eq!(
        income_rule.evaluate(&store, Some(ValueKind::Budgeted)),
        Money::new_dollars(5000, Currency::SEK)
    );
    assert_eq!(
        remaining_rule.evaluate(&store, Some(ValueKind::Budgeted)),
        Money::new_dollars(1000, Currency::SEK)
    );
}
