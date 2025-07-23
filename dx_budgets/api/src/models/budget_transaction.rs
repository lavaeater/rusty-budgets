#[cfg(feature = "server")]
use crate::models::budget_item::BudgetItem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::WeldsModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(WeldsModel))]
#[cfg_attr(feature = "server", welds(table = "budget_transactions"))]
#[cfg_attr(feature = "server", welds(BelongsTo(to_budget_item, BudgetItem, "to_budget_item")))]
#[cfg_attr(feature = "server", welds(BelongsTo(from_budget_item, BudgetItem, "from_budget_item")))]
#[cfg_attr(feature = "server", welds(BelongsTo(created_by, BudgetItem, "created_by")))]
#[cfg_attr(feature = "server", welds(BeforeCreate(before_create)))]
#[cfg_attr(feature = "server", welds(BeforeUpdate(before_update)))]
pub struct BudgetTransaction {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub text: String,
    pub amount: f32,
    pub from_budget_item: Option<Uuid>,
    pub to_budget_item: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}



#[cfg(feature = "server")]
pub fn before_create(budget_transaction: &mut BudgetTransaction) -> welds::errors::Result<()>{
    budget_transaction.id = Uuid::new_v4();
    budget_transaction.created_at = chrono::Utc::now().naive_utc();
    budget_transaction.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

#[cfg(feature = "server")]
pub fn before_update(budget_transaction: &mut BudgetTransaction) -> welds::errors::Result<()>{
    budget_transaction.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

