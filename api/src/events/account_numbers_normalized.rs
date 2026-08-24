use crate::cqrs::framework::{Aggregate, CommandError, DomainEvent};
use crate::models::{Budget, normalize_account_number};
use cqrs_macros::DomainEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// One-off cleanup for a data-entry bug: the Skandiabanken import stored the
/// primary account's number as the bank formats it for display
/// (`9159-482.485-3`) while a counterpart account discovered via a transfer
/// description was already stripped to digits (`91594824853`) — the same
/// physical account ended up as two different [`crate::models::BankAccount`]
/// rows. Deriving the merge from current state at apply time (instead of
/// carrying a payload) keeps this replay-safe: given the same prior events,
/// grouping accounts by their normalized number always produces the same
/// result.
#[derive(Debug, Clone, Serialize, Deserialize, DomainEvent)]
#[domain_event(aggregate = "Budget", command_fn = "normalize_account_numbers")]
pub struct AccountNumbersNormalized {
    pub budget_id: Uuid,
}

impl AccountNumbersNormalizedHandler for Budget {
    fn apply_normalize_account_numbers(&mut self, event: &AccountNumbersNormalized) -> Uuid {
        // Group existing accounts by their normalized number, choosing the
        // lowest id in each group as the surviving canonical account —
        // deterministic, so replay always picks the same one.
        let mut groups: HashMap<String, Vec<Uuid>> = HashMap::new();
        for account in &self.accounts {
            groups
                .entry(normalize_account_number(&account.account_number))
                .or_default()
                .push(account.id);
        }

        // account_id -> canonical (account_id, normalized_number) it merges into.
        let mut canonical_by_account: HashMap<Uuid, (Uuid, String)> = HashMap::new();
        for (normalized, mut ids) in groups {
            ids.sort();
            let canonical_id = ids[0];
            for id in ids {
                canonical_by_account.insert(id, (canonical_id, normalized.clone()));
            }
        }

        // old raw account_number string -> canonical normalized number, for
        // rewriting transactions/transfer rules that reference accounts by
        // their (possibly unnormalized) number rather than by id.
        let mut number_rewrites: HashMap<String, String> = HashMap::new();
        for account in &self.accounts {
            if let Some((_, canonical_number)) = canonical_by_account.get(&account.id)
                && *canonical_number != account.account_number
            {
                number_rewrites.insert(account.account_number.clone(), canonical_number.clone());
            }
        }

        if number_rewrites.is_empty() {
            return event.budget_id;
        }

        self.accounts
            .retain(|account| canonical_by_account[&account.id].0 == account.id);
        for account in &mut self.accounts {
            if let Some(canonical) = number_rewrites.get(&account.account_number) {
                account.account_number = canonical.clone();
            }
        }

        for period in &mut self.periods {
            for tx in &mut period.transactions {
                if let Some(canonical) = number_rewrites.get(&tx.account_number) {
                    tx.account_number = canonical.clone();
                }
            }
        }

        self.transfer_rules = self
            .transfer_rules
            .drain()
            .map(|mut rule| {
                if let Some(canonical) = number_rewrites.get(&rule.outgoing_account) {
                    rule.outgoing_account = canonical.clone();
                }
                if let Some(canonical) = number_rewrites.get(&rule.incoming_account) {
                    rule.incoming_account = canonical.clone();
                }
                rule
            })
            .collect();

        event.budget_id
    }

    fn normalize_account_numbers_impl(&self) -> Result<AccountNumbersNormalized, CommandError> {
        Ok(AccountNumbersNormalized { budget_id: self.id })
    }
}
