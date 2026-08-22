//! Full budget export/import — a portable JSON dump of an entire budget
//! (accounts, tags, match rules, transfer rules, items, periods, every
//! transaction) for moving a budget to another instance of the app.
//!
//! Unlike [`crate::rules_export`], which merges tags/rules by name into an
//! existing budget, this is a full-fidelity snapshot: importing always
//! creates a brand new budget, never merges into one that already exists.

use crate::models::Budget;
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Rewrites a parsed export for its new home: a fresh id (so it can't
/// collide with the budget it came from — including re-importing onto the
/// same instance it was exported from), owned by `user_id`, and never
/// marked default (importing must not silently swap the user's active
/// budget out from under them).
///
/// Every `BankTransaction.id` is also reminted. Unlike tag/item/rule ids —
/// meaningful only *within* this budget's own JSON — transaction ids are
/// primary keys in the shared, cross-budget `transactions` table (see
/// `PgRuntime::load`/`snapshot`), so keeping the source's ids would collide
/// with the very rows this budget was exported from. `TransactionAllocation`
/// and `rejected_transfer_pairs` reference transactions by id and are
/// remapped alongside them so those links stay correct. Every other id
/// (tags, items, rules, transfer rules, periods, accounts) is kept exactly
/// as exported.
pub fn prepare_imported_budget(mut budget: Budget, user_id: Uuid) -> Budget {
    budget.id = Uuid::new_v4();
    budget.user_id = user_id;
    budget.default_budget = false;
    budget.updated_at = Utc::now();
    budget.last_event = 0;
    budget.version = 0;

    let mut tx_id_map: HashMap<Uuid, Uuid> = HashMap::new();
    for period in &mut budget.periods {
        for tx in &mut period.transactions {
            let new_id = Uuid::new_v4();
            tx_id_map.insert(tx.id, new_id);
            tx.id = new_id;
        }
    }
    for period in &mut budget.periods {
        for allocation in &mut period.allocations {
            if let Some(&new_id) = tx_id_map.get(&allocation.transaction_id) {
                allocation.transaction_id = new_id;
            }
        }
    }
    budget.rejected_transfer_pairs = budget
        .rejected_transfer_pairs
        .iter()
        .map(|(a, b)| {
            (
                tx_id_map.get(a).copied().unwrap_or(*a),
                tx_id_map.get(b).copied().unwrap_or(*b),
            )
        })
        .collect();

    budget
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Currency;

    #[test]
    fn import_gets_a_fresh_id_and_owner_but_keeps_the_rest() {
        let mut original = Budget::new(Uuid::new_v4());
        original.name = "Original".to_string();
        original.user_id = Uuid::new_v4();
        original.default_budget = true;
        original.currency = Currency::EUR;

        // Round-trip through JSON, exactly like a real export/import.
        let exported = serde_json::to_string(&original).unwrap();
        let parsed: Budget = serde_json::from_str(&exported).unwrap();

        let new_user = Uuid::new_v4();
        let imported = prepare_imported_budget(parsed, new_user);

        assert_ne!(imported.id, original.id, "must not collide with the source budget");
        assert_eq!(imported.user_id, new_user);
        assert!(!imported.default_budget, "must not silently become the active budget");
        assert_eq!(imported.last_event, 0);
        assert_eq!(imported.version, 0);
        assert_eq!(imported.name, "Original");
        assert_eq!(imported.currency, Currency::EUR);
    }

    #[test]
    fn transaction_ids_are_reminted_and_references_follow() {
        use crate::models::{BankTransaction, Money, TransactionAllocation};

        let mut original = Budget::new(Uuid::new_v4());
        let tx = BankTransaction::new(
            Uuid::new_v4(),
            "1234",
            Money::new_cents(-500, Currency::SEK),
            Money::new_cents(10_000, Currency::SEK),
            "Ica",
            Utc::now(),
        );
        let old_tx_id = tx.id;
        let allocation = TransactionAllocation::new(
            old_tx_id,
            Uuid::new_v4(),
            Money::new_cents(-500, Currency::SEK),
            "Mat".to_string(),
        );
        original.periods[0].transactions.push(tx);
        original.periods[0].allocations.push(allocation);
        original.rejected_transfer_pairs.insert((old_tx_id, Uuid::new_v4()));

        let imported = prepare_imported_budget(original, Uuid::new_v4());

        let new_tx_id = imported.periods[0].transactions[0].id;
        assert_ne!(new_tx_id, old_tx_id, "transaction id must be reminted");
        assert_eq!(
            imported.periods[0].allocations[0].transaction_id, new_tx_id,
            "allocation must follow the transaction's new id"
        );
        assert!(
            imported.rejected_transfer_pairs.iter().any(|(a, b)| *a == new_tx_id || *b == new_tx_id),
            "rejected transfer pair must follow the transaction's new id"
        );
    }
}
