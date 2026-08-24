use crate::models::BankTransaction;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};
use uuid::Uuid;

/// A learned pattern for a recurring internal-transfer pair (e.g. a monthly
/// savings contribution, or a routine transfer between two of the user's own
/// accounts), created automatically whenever the user resolves a
/// [`crate::models::Budget::potential_internal_transfers`] pair by hand.
///
/// Matches are only ever **suggested** to the user, never silently applied —
/// see [`crate::models::Budget::suggested_transfer_resolutions`]. This moves
/// money between accounts/tags, which isn't something to auto-apply the way
/// an ordinary [`crate::models::MatchRule`] auto-tags a bill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRule {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Account the "outgoing" leg (the one that gets tagged or ignored) was
    /// paid from.
    pub outgoing_account: String,
    /// Account the "incoming" leg (always ignored) landed in.
    pub incoming_account: String,
    pub outgoing_key: Vec<String>,
    pub incoming_key: Vec<String>,
    /// `None` replays as a plain internal transfer (both legs ignored);
    /// `Some` replays as a savings contribution (outgoing tagged with this
    /// tag, incoming ignored) — same shape as `resolve_transfer_pair`.
    pub tag_id: Option<Uuid>,
}

impl Hash for TransferRule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.outgoing_account.hash(state);
        self.incoming_account.hash(state);
        self.outgoing_key.hash(state);
        self.incoming_key.hash(state);
        self.tag_id.hash(state);
    }
}

impl PartialEq for TransferRule {
    fn eq(&self, other: &Self) -> bool {
        self.outgoing_account == other.outgoing_account
            && self.incoming_account == other.incoming_account
            && self.outgoing_key == other.outgoing_key
            && self.incoming_key == other.incoming_key
            && self.tag_id == other.tag_id
    }
}

impl Eq for TransferRule {}

impl TransferRule {
    fn key_matches(key: &[String], description: &str) -> bool {
        if key.is_empty() {
            return false;
        }
        let tokens = crate::models::match_rule::tokenize_description(description);
        key.iter().all(|token| tokens.contains(token))
    }

    /// If this rule recognizes `a`/`b` as one instance of its learned
    /// pattern, returns the `(outgoing_id, incoming_id)` orientation to
    /// replay the resolution with — checked in both orders since a
    /// candidate pair from `potential_internal_transfers` isn't guaranteed
    /// to list the outgoing leg first.
    pub fn matches_pair(&self, a: &BankTransaction, b: &BankTransaction) -> Option<(Uuid, Uuid)> {
        let is_outgoing = |tx: &BankTransaction| {
            tx.account_number == self.outgoing_account
                && Self::key_matches(&self.outgoing_key, &tx.description)
        };
        let is_incoming = |tx: &BankTransaction| {
            tx.account_number == self.incoming_account
                && Self::key_matches(&self.incoming_key, &tx.description)
        };

        if is_outgoing(a) && is_incoming(b) {
            return Some((a.id, b.id));
        }
        if is_outgoing(b) && is_incoming(a) {
            return Some((b.id, a.id));
        }
        None
    }
}
