use core::fmt::Display;
use crate::models::{BankTransaction, BudgetItem};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use dioxus::logger::tracing;
use once_cell::sync::Lazy;
use uuid::Uuid;
use crate::models::actual_item::ActualItem;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct  MatchRule {
    pub transaction_key: Vec<String>,
    pub item_key: Vec<String>,
    pub always_apply: bool
}

impl Display for MatchRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "MatchRule {{ transaction_key: {:?}, item_name: {:?}, always_apply: {} }}", self.transaction_key, self.item_key, self.always_apply)
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
    // set.insert("överföring");
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

    pub fn matches_actual(&self, actual: &ActualItem) -> bool {
        let tokenized_item_name = tokenize_description(&actual.item_name);
        self.item_key == tokenized_item_name
    }

    pub fn matches_item(&self, item: &BudgetItem) -> bool {
        let tokenized_item_name = tokenize_description(&item.name);
        self.item_key == tokenized_item_name
    }
    
    pub fn create_rule_for_transaction_and_item(transaction: &BankTransaction, item: &ActualItem) -> MatchRule {
        let transaction_key = Self::create_transaction_key(transaction);
        MatchRule {
            transaction_key,
            item_key: Self::create_item_key(item),
            always_apply: true
        }
    }
    
    pub fn create_item_key(item: &ActualItem) -> Vec<String> {
        tokenize_description(&item.item_name)
    }
    
    pub fn create_transaction_key(transaction: &BankTransaction) -> Vec<String> {
        tokenize_description(&transaction.description)
    }
}

