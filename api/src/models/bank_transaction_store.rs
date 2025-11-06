use core::fmt::Display;
use crate::models::{BankTransaction, BudgetItem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use once_cell::sync::Lazy;
use uuid::Uuid;
use crate::models::actual_item::ActualItem;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct BankTransactionStore {
    hashes: HashSet<u64>,                  // uniqueness check
    by_id: HashMap<Uuid, BankTransaction>, // fast lookup
    ignored: HashMap<Uuid, BankTransaction>
}

impl BankTransactionStore {
    pub fn list_ignored_transactions(&self) -> Vec<BankTransaction> {
        self.ignored.values().cloned().collect()
    }
}

impl BankTransactionStore {
    pub fn list_transactions_for_item(&self, item_id: &Uuid, sorted: bool) -> Vec<&BankTransaction> {
        let mut transactions = self.by_id.values().filter(|tx| tx.budget_item_id == Some(*item_id)).collect::<Vec<_>>();
        if sorted {
            transactions.sort_by_key(|tx| tx.date);
        }
        transactions
    }
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
    
    pub fn ignore_transaction(&mut self, tx_id: &Uuid) -> bool {
        if let Some(mut transaction) = self.by_id.remove(tx_id) {
            transaction.ignored = true;
            transaction.budget_item_id = None;
            
            self.ignored.insert(*tx_id, transaction);
            true
        } else {
            false
        }
    }

    pub fn check_hash(&self, hash: &u64) -> bool {
        self.hashes.contains(hash)
    }

    pub fn can_insert(&self, hash: &u64) -> bool {
        !self.hashes.contains(hash)
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

    pub fn list_transactions(&self, sorted: bool) -> Vec<&BankTransaction> {
        if sorted {
            let mut transactions = self.by_id.values().collect::<Vec<_>>();
            transactions.sort_by_key(|tx| tx.date);
            transactions
        } else {
            self.by_id.values().collect()
        }
    }

    pub fn list_transactions_for_connection(&self) -> Vec<BankTransaction> {
        let mut transactions = self
            .by_id
            .values()
            .filter(|tx| tx.budget_item_id.is_none())
            .cloned()
            .collect::<Vec<_>>();

        // Sort transactions by date, then by absolute amount (descending), then by amount (ascending)
        // This groups transactions with opposite amounts together
        transactions.sort_by(|a, b| {
            a.date.cmp(&b.date)
                .then_with(|| {
                    // Compare absolute amounts in reverse order (largest absolute amounts first)
                    b.amount.abs().partial_cmp(&a.amount.abs()).unwrap()
                })
                .then_with(|| {
                    // Then by actual amount (this will group positive and negative amounts together)
                    a.amount.partial_cmp(&b.amount).unwrap()
                })
                .then_with(|| a.description.cmp(&b.description))
        });

        transactions
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct MatchRule {
    pub transaction_key: Vec<String>,
    pub item_name: String,
    pub always_apply: bool
}

impl Display for MatchRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MatchRule {{ transaction_key: {:?}, item_name: {}, always_apply: {} }}", self.transaction_key, self.item_name, self.always_apply)
    }
}

// Default stopwords to filter out from tokenized descriptions
static DEFAULT_STOPWORDS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    /*
                "orebro",
            "vastha,",

     */
    let mut set = HashSet::new();
    set.insert("kontaktlös");
    set.insert("zettle");
    set.insert("zettle_*");
    set.insert("överföring");
    set.insert("autogiro");
    set.insert("orebro");
    set.insert("vastha");
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
        self.transaction_key == tokenized_transaction_description
    }

    pub fn matches_item(&self, item: &ActualItem) -> bool {
        item.budget_item.as_ref().name.contains(&self.item_name)
    }
    
    pub fn create_rule_for_transaction_and_item(transaction: &BankTransaction, item: &BudgetItem) -> MatchRule {
        let transaction_key = Self::create_transaction_key(transaction);
        MatchRule {
            transaction_key,
            item_name: item.name.clone(),
            always_apply: true
        }
    }
    
    pub fn create_transaction_key(transaction: &BankTransaction) -> Vec<String> {
        tokenize_description(&transaction.description)
    }
}

