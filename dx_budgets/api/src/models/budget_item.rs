#[cfg(feature = "server")]
use crate::models::budget::Budget;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[cfg(feature = "server")]
use welds::WeldsModel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(WeldsModel))]
#[cfg_attr(feature = "server", welds(table = "budget_items"))]
#[cfg_attr(feature = "server", welds(BelongsTo(budget, Budget, "budget_id")))]
pub struct BudgetItem {
    #[cfg_attr(feature = "server", welds(primary_key))]
    pub id: Uuid,
    pub name: String,
    pub amount: f32,
    pub budget_id: Uuid,
}
