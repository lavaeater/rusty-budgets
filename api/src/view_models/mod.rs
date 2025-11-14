use crate::models::{ActualItem, BankTransaction, Budget, Currency, MonthBeginsOn, PeriodId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct BudgetViewModel {
    id: Uuid,
    name: String,
    month_begins_on: MonthBeginsOn,
    period_id: PeriodId,
    actual_items: Vec<ActualItem>,
    to_connect: Vec<BankTransaction>,
    ignored_transactions: Vec<BankTransaction>,
    currency: Currency,
}

impl BudgetViewModel {
    pub fn from_budget(budget: Budget, period_id: PeriodId) -> Self {
        Self {
            id: budget.id,
            name: budget.name.clone(),
            month_begins_on: budget.month_begins_on(),
            period_id,
            actual_items: budget.with_period(period_id).all_actual_items(),
            to_connect: budget.list_transactions_for_connection(period_id),
            ignored_transactions: budget.list_ignored_transactions(period_id),
            currency: budget.currency,
        }
    }
}
