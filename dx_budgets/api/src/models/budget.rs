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
pub struct Budget {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub name: String,
    pub default_budget: bool,
    pub user_id: Uuid,
}
