use crate::User;
#[cfg(feature = "server")]
use crate::models::budget_item::BudgetItem;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::WeldsModel;


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[cfg_attr(feature = "server", derive(WeldsModel))]
#[cfg_attr(feature = "server", welds(table = "budgets"))]
#[cfg_attr(feature = "server", welds(BelongsTo(user, User, "user_id")))]
#[cfg_attr(
    feature = "server",
    welds(HasMany(budget_items, BudgetItem, "budget_id"))
)]
#[cfg_attr(feature = "server", welds(BeforeCreate(before_create)))]
#[cfg_attr(feature = "server", welds(BeforeUpdate(before_update)))]
pub struct Budget {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub user_id: Uuid,
}

#[cfg(feature = "server")]
pub fn before_create(budget: &mut Budget) -> welds::errors::Result<()>{
    budget.id = Uuid::new_v4();
    budget.created_at = chrono::Utc::now().naive_utc();
    budget.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

#[cfg(feature = "server")]
pub fn before_update(budget: &mut Budget) -> welds::errors::Result<()>{
    budget.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}
