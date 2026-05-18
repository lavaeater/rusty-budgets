use crate::models::money::Money;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransactionAllocation {
    pub id: Uuid,
    pub transaction_id: Uuid,
    pub actual_id: Uuid,
    pub amount: Money,
    pub tag: String,
}

impl TransactionAllocation {
    pub fn new(transaction_id: Uuid, actual_id: Uuid, amount: Money, tag: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            transaction_id,
            actual_id,
            amount,
            tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Currency;

    #[test]
    fn new_assigns_unique_id() {
        let tx_id = Uuid::new_v4();
        let actual_id = Uuid::new_v4();
        let amount = Money::new_dollars(100, Currency::SEK);
        let a = TransactionAllocation::new(tx_id, actual_id, amount, "groceries".to_string());
        let b = TransactionAllocation::new(tx_id, actual_id, amount, "groceries".to_string());
        assert_ne!(a.id, b.id);
        assert_eq!(a.transaction_id, tx_id);
        assert_eq!(a.actual_id, actual_id);
        assert_eq!(a.amount, amount);
        assert_eq!(a.tag, "groceries");
    }

    #[test]
    fn serde_round_trip() {
        let a = TransactionAllocation::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Money::new_dollars(50, Currency::SEK),
            "rent".to_string(),
        );
        let json = serde_json::to_string(&a).unwrap();
        let back: TransactionAllocation = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }
}
