use crate::models::{BankTransaction, BudgetItem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use once_cell::sync::Lazy;
use uuid::Uuid;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionStore {
    hashes: HashSet<u64>,                  // uniqueness check
    by_id: HashMap<Uuid, BankTransaction>, // fast lookup
}

impl BankTransactionStore {
    pub fn clear(&mut self) {
        self.hashes.clear();
        self.by_id.clear();
    }
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }

    pub fn insert(&mut self, transaction: BankTransaction) -> bool {
        let mut hasher = DefaultHasher::new();
        transaction.hash(&mut hasher);

        if self.hashes.insert(hasher.finish()) {
            self.by_id.insert(transaction.id, transaction);
            true
        } else {
            false
        }
    }

    pub fn remove(&mut self, id: Uuid) -> bool {
        if let Some(transaction) = self.by_id.remove(&id) {
            let mut hasher = DefaultHasher::new();
            transaction.hash(&mut hasher);
            self.hashes.remove(&hasher.finish())
        } else {
            false
        }
    }

    pub fn check_hash(&self, hash: &u64) -> bool {
        self.hashes.contains(hash)
    }

    pub fn can_insert(&self, hash: &u64) -> bool {
        !self.check_hash(hash)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut BankTransaction> {
        self.by_id.get_mut(id)
    }

    pub fn get(&self, id: &Uuid) -> Option<&BankTransaction> {
        self.by_id.get(id)
    }

    pub fn contains(&self, id: &Uuid) -> bool {
        self.by_id.contains_key(id)
    }

    pub fn list_transactions(&self) -> Vec<&BankTransaction> {
        let mut transactions = self.by_id.values().collect::<Vec<_>>();
        transactions.sort_by_key(|tx| tx.date);
        transactions
    }

    pub fn list_transactions_for_connection(&self) -> Vec<&BankTransaction> {
        let mut transactions = self
            .by_id
            .values()
            .filter(|tx| tx.budget_item_id.is_none())
            .collect::<Vec<_>>();
        transactions.sort_by_key(|tx| tx.date);
        transactions
    }

    fn suggest_item_for(_description: &str) -> Option<Uuid> {
        // TODO: Implement matching against rules / heuristics
        None
    }
}

pub struct MatchRule {
    pub transaction_key: Vec<String>,
    pub item_name: String,
    pub item_id: Option<Uuid>,
    pub always_apply: bool
}

// Default stopwords to filter out from tokenized descriptions
static DEFAULT_STOPWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();
    set.insert("kontaktlös");
    set.insert("zettle");
    set.insert("zettle_*");
    set.insert("överföring");
    set.insert("autogiro");
    set
});

/// Checks if a string matches a date pattern (YYYY-MM-DD or similar)
fn is_date_pattern(s: &str) -> bool {
    // Check for patterns like 2025-09-30, 2025/09/30, 20250930
    if s.len() < 8 {
        return false;
    }

    // Pattern: YYYY-MM-DD or YYYY/MM/DD
    if s.len() == 10 && (s.contains('-') || s.contains('/')) {
        let parts: Vec<&str> = if s.contains('-') {
            s.split('-').collect()
        } else {
            s.split('/').collect()
        };

        if parts.len() == 3 {
            return parts[0].len() == 4 && parts[0].chars().all(|c| c.is_numeric())
                && parts[1].len() == 2 && parts[1].chars().all(|c| c.is_numeric())
                && parts[2].len() == 2 && parts[2].chars().all(|c| c.is_numeric());
        }
    }

    // Pattern: YYYYMMDD
    if s.len() == 8 && s.chars().all(|c| c.is_numeric()) {
        return true;
    }

    false
}

/// Checks if a string is purely numeric (account numbers, transaction IDs, etc.)
fn is_numeric_only(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_numeric())
}

/// Tokenizes a bank transaction description and filters out noise
///
/// # Arguments
/// * `description` - The transaction description string
///
/// # Returns
/// A vector of filtered tokens (lowercase, no dates, no pure numbers, no stopwords)
///
/// # Examples
/// ```
/// let tokens = tokenize_description("2025-09-30 WILLYS OREBRO VASTHA, OREBRO");
/// // Returns: ["willys", "orebro", "vastha,", "orebro"]
/// ```
pub fn tokenize_description(description: &str) -> Vec<String> {
    /*
    Example strings:
    2025-09-30 WILLYS OREBRO VASTHA, OREBRO
    Överföring 9151 1421586
    2025-09-27 kontaktlös ZETTLE_*ELINAS MARKNAD, GRODINGE
    2025-09-26 kontaktlös KREATIMA STOCKHOLM, STOCKHOLM
    Autogiro Qliro
    LÖN
     */

    description
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .filter(|token| {
            // Filter out dates
            !is_date_pattern(token) &&
                // Filter out pure numeric strings
                !is_numeric_only(token) &&
                // Filter out stopwords
                !DEFAULT_STOPWORDS.contains(token.as_str())
        })
        .collect()
}

/// Tokenizes with custom stopwords
///
/// # Arguments
/// * `description` - The transaction description string
/// * `custom_stopwords` - Additional stopwords to filter out
///
/// # Returns
/// A vector of filtered tokens
pub fn tokenize_description_with_stopwords(
    description: &str,
    custom_stopwords: &HashSet<String>,
) -> Vec<String> {
    description
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .filter(|token| {
            !is_date_pattern(token) &&
                !is_numeric_only(token) &&
                !DEFAULT_STOPWORDS.contains(token.as_str()) &&
                !custom_stopwords.contains(token)
        })
        .collect()
}

impl MatchRule {
    pub fn matches_transaction(&self, transaction: &BankTransaction) -> bool {
        let tokenized_transaction_description = tokenize_description(&transaction.description);
        if self.transaction_key.iter().all(|key| tokenized_transaction_description.contains(key)) {
            return true;
        }
        false
    }

    pub fn matches_item(&self, item: &BudgetItem) -> bool {
        let name_match = item.name.contains(&self.item_name);
        match self.item_id {
            Some(id) => name_match || item.id == id,
            None => name_match
        }
    }
    
    pub fn create_rule_for_transaction_and_item(transaction: &BankTransaction, item: &BudgetItem) -> MatchRule {
        MatchRule {
            transaction_key: tokenize_description(&transaction.description),
            item_name: item.name.clone(),
            item_id: Some(item.id),
            always_apply: false
        }
    }
}

