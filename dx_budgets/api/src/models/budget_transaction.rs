use joydb::Model;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Model)]
pub struct BudgetTransaction {
    pub id: Uuid,
    pub text: String,
    pub amount: f32,
    pub from_budget_item: Option<Uuid>,
    pub to_budget_item: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl BudgetTransaction {
    pub fn new_from_user(text: &str, amount: f32, from_budget_item: Option<Uuid>, to_budget_item: Uuid, created_by: Uuid) -> BudgetTransaction {
        BudgetTransaction {
            id: Uuid::new_v4(),
            text: text.to_string(),
            amount,
            to_budget_item,
            from_budget_item,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}




pub fn before_create(budget_transaction: &mut BudgetTransaction) -> welds::errors::Result<()>{
    budget_transaction.id = Uuid::new_v4();
    budget_transaction.created_at = chrono::Utc::now().naive_utc();
    budget_transaction.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}


pub fn before_update(budget_transaction: &mut BudgetTransaction) -> welds::errors::Result<()>{
    budget_transaction.updated_at = chrono::Utc::now().naive_utc();
    Ok(())
}

