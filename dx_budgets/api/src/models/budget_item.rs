#[cfg(feature = "server")]
use crate::models::budget::Budget;
#[cfg(feature = "server")]
use crate::models::budget_transaction::BudgetTransaction;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::WeldsModel;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(WeldsModel))]
#[cfg_attr(feature = "server", welds(table = "budget_items"))]
#[cfg_attr(feature = "server", welds(BelongsTo(budget, Budget, "budget_id")))]
#[cfg_attr(
    feature = "server",
    welds(HasMany(outgoing_budget_transactions, BudgetTransaction, "from_budget_item"))
)]
#[cfg_attr(
    feature = "server",
    welds(HasMany(incoming_budget_transactions, BudgetTransaction, "to_budget_item"))
)]
#[cfg_attr(feature = "server", welds(BeforeCreate(before_create)))]
#[cfg_attr(feature = "server", welds(BeforeUpdate(before_update)))]
pub struct BudgetItem {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub name: String,
    pub expected_at: chrono::NaiveDate,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
    pub budget_id: Uuid,
}

impl BudgetItem {
    pub fn new_from_user(budget_id: Uuid, name: &str, expected_at: chrono::NaiveDate, created_by: Uuid) -> BudgetItem {
        BudgetItem {
            budget_id,
            name: name.to_string(),
            expected_at,
            created_by,
            ..Default::default()
        }
    }
}

#[cfg(feature = "server")]
pub fn before_create(budget_item: &mut BudgetItem) -> welds::errors::Result<()> {
    budget_item.id = Uuid::new_v4();
    budget_item.created_at = chrono::Utc::now().naive_utc();
    budget_item.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

#[cfg(feature = "server")]
pub fn before_update(budget_item: &mut BudgetItem) -> welds::errors::Result<()> {
    budget_item.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}
