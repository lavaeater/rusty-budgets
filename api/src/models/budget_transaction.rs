use std::hash::{Hash, Hasher};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use joydb::Model;
use crate::models::budget_transaction_type::BudgetTransactionType;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Model)]
pub struct BudgetTransaction {
    pub id: Uuid,
    pub text: String,
    pub transaction_type: BudgetTransactionType,
    pub amount: f32,
    pub from_budget_item: Option<Uuid>,
    pub to_budget_item: Uuid,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub created_by: Uuid,
}

impl PartialEq for BudgetTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for BudgetTransaction {}

impl Hash for BudgetTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl BudgetTransaction {
    pub fn new(
        text: &str,
        transaction_type: BudgetTransactionType,
        amount: f32,
        from_budget_item: Option<Uuid>,
        to_budget_item: Uuid,
        created_by: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            text: text.to_string(),
            transaction_type,
            amount,
            to_budget_item,
            from_budget_item,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
            created_by,
        }
    }
}