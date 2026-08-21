use crate::models::actual_item::ActualItem;
use crate::models::{BankTransaction, BudgetItem};
use core::fmt::Display;
use dioxus::logger::tracing;
use iter_tools::Itertools;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;
use once_cell::sync::Lazy;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRule {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub transaction_key: Vec<String>,
    pub item_key: Vec<String>,
    pub always_apply: bool,
    #[serde(default)]
    pub tag_id: Option<Uuid>,
}

impl Hash for MatchRule {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.transaction_key.hash(state);
        self.item_key.hash(state);
        self.always_apply.hash(state);
        self.tag_id.hash(state);
    }
}

impl PartialEq for MatchRule {
    fn eq(&self, other: &Self) -> bool {
        self.transaction_key == other.transaction_key
            && self.item_key == other.item_key
            && self.always_apply == other.always_apply
            && self.tag_id == other.tag_id
    }
}

impl Eq for MatchRule {}

impl Display for MatchRule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "MatchRule {{ transaction_key: {:?}, item_name: {:?}, always_apply: {} }}",
            self.transaction_key, self.item_key, self.always_apply
        )
    }
}

// Default stopwords to filter out from tokenized descriptions
static DEFAULT_STOPWORDS: std::sync::LazyLock<HashSet<&'static str>> = std::sync::LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert("kontaktlös");
    set.insert("zettle");
    set.insert("zettle_*");
    set.insert("autogiro");
    set
});

// Default place names to filter out from tokenized descriptions
static DEFAULT_PLACE_NAMES: std::sync::LazyLock<HashSet<&'static str>> = std::sync::LazyLock::new(|| {
    let mut set = HashSet::new();
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
            return parts[0].len() == 4
                && parts[0].chars().all(char::is_numeric)
                && parts[1].len() == 2
                && parts[1].chars().all(char::is_numeric)
                && parts[2].len() == 2
                && parts[2].chars().all(char::is_numeric);
        }
    }

    // Pattern: YYYYMMDD
    if s.len() == 8 && s.chars().all(char::is_numeric) {
        return true;
    }

    false
}

/// Strips leading/trailing punctuation that bank exports glue onto tokens
/// (most commonly a trailing comma from "CITY, CITY"-style address lists),
/// so e.g. "noodles," and "noodles" tokenize identically instead of being
/// treated as different match-rule terms. Deliberately narrow — it must not
/// touch the internal `_`/`*` in tokens like "zettle_*elinas".
fn strip_punctuation(token: &str) -> &str {
    token.trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':'))
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
        .map(strip_punctuation)
        .filter(|token| !token.is_empty())
        .filter(|token| {
            // Filter out dates
            !is_date_pattern(token) &&
                // Filter out stopwords
                !DEFAULT_STOPWORDS.contains(*token)
            && !DEFAULT_PLACE_NAMES.contains(*token)
        })
        .map(std::string::ToString::to_string)
        .collect()
}

pub fn strip_dates(description: &str) -> String {
    description
        .split_whitespace()
        .map(std::string::ToString::to_string)
        .filter(|token| {
            // Filter out dates
            !is_date_pattern(token)
        })
        .join(" ")
}

/// Tokenizes with custom stopwords
///
/// # Arguments
/// * `description` - The transaction description string
/// * `custom_stopwords` - Additional stopwords to filter out
///
/// # Returns
/// A vector of filtered tokens
#[allow(clippy::implicit_hasher)]
pub fn tokenize_description_with_stopwords(
    description: &str,
    custom_stopwords: &HashSet<String>,
) -> Vec<String> {
    description
        .to_lowercase()
        .split_whitespace()
        .map(strip_punctuation)
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !is_date_pattern(token)
                && !DEFAULT_STOPWORDS.contains(*token)
                && !custom_stopwords.contains(*token)
        })
        .map(std::string::ToString::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_description_new_china_trading() {
        let description = "2025-11-26 NEW CHINA TRADING, OREBRO";
        let tokens = tokenize_description(description);

        // Date "2025-11-26" should be filtered out
        // "OREBRO" should be filtered out (stopword)
        // Remaining tokens should be lowercase, with the trailing comma from
        // "TRADING, OREBRO" stripped so "trading," and "trading" tokenize
        // identically.
        assert_eq!(tokens, vec!["new", "china", "trading"]);
    }

    /// Table-driven coverage of the tokenizer's documented behaviour. This is
    /// the heart of auto-categorisation (a `MatchRule.transaction_key` is just a
    /// tokenised description), so the contract is locked down exhaustively.
    #[test]
    fn test_tokenize_description_table() {
        // (input, expected tokens)
        let cases: &[(&str, Vec<&str>)] = &[
            // Lowercasing.
            ("LÖN", vec!["lön"]),
            // ISO date (YYYY-MM-DD) dropped. "VASTHA," loses its trailing
            // comma before stopword filtering, so it now matches the
            // "vastha" place-name stopword the same as "OREBRO" does —
            // both are dropped.
            (
                "2025-09-30 WILLYS OREBRO VASTHA, OREBRO",
                vec!["willys"],
            ),
            // Stopword "autogiro" dropped.
            ("Autogiro Qliro", vec!["qliro"]),
            // "kontaktlös" is a stopword; "zettle_*" only matches verbatim, so
            // "zettle_*elinas" survives (its internal `_`/`*` are untouched —
            // only leading/trailing punctuation is stripped). "MARKNAD,"
            // loses its trailing comma.
            (
                "2025-09-27 kontaktlös ZETTLE_*ELINAS MARKNAD, GRODINGE",
                vec!["zettle_*elinas", "marknad", "grodinge"],
            ),
            // Numbers that are NOT dates are kept (only date-shaped tokens drop):
            // "9151"/"1421586" are too short to be a compact YYYYMMDD date.
            (
                "Överföring 9151 1421586",
                vec!["överföring", "9151", "1421586"],
            ),
            // Compact YYYYMMDD (8 digits) is recognised as a date and dropped.
            ("20250930 TEST", vec!["test"]),
            // Slash-separated date dropped.
            ("2025/09/30 COOP", vec!["coop"]),
            // Empty input yields no tokens.
            ("", vec![]),
            // Whitespace-only input yields no tokens.
            ("   ", vec![]),
        ];

        for (input, expected) in cases {
            let got = tokenize_description(input);
            let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
            assert_eq!(&got, &expected, "tokenize_description({input:?})");
        }
    }

    #[test]
    fn test_is_date_pattern() {
        assert!(is_date_pattern("2025-09-30"));
        assert!(is_date_pattern("2025/09/30"));
        assert!(is_date_pattern("20250930"));
        // Too short.
        assert!(!is_date_pattern("2025-9-3"));
        assert!(!is_date_pattern("9151"));
        // Right length but not all numeric.
        assert!(!is_date_pattern("2025-ab-30"));
        assert!(!is_date_pattern("hello123"));
    }

    #[test]
    fn test_tokenize_with_custom_stopwords() {
        let mut custom = HashSet::new();
        custom.insert("willys".to_string());
        let tokens = tokenize_description_with_stopwords("WILLYS mat", &custom);
        // "willys" filtered by the custom stopword, "mat" survives.
        assert_eq!(tokens, vec!["mat"]);
    }

    #[test]
    fn test_matches_transaction_requires_all_tokens() {
        let now = chrono::Utc::now();
        let tx = BankTransaction::new(
            Uuid::new_v4(),
            "acc1",
            crate::models::Money::new_dollars(-100, crate::models::Currency::SEK),
            crate::models::Money::new_dollars(900, crate::models::Currency::SEK),
            "WILLYS OREBRO MAT",
            now,
        );

        // All tokens present -> matches.
        let rule = MatchRule {
            id: Uuid::new_v4(),
            transaction_key: vec!["willys".to_string(), "mat".to_string()],
            item_key: Vec::new(),
            always_apply: true,
            tag_id: None,
        };
        assert!(rule.matches_transaction(&tx));

        // A token that isn't in the (tokenised) description -> no match.
        let rule = MatchRule {
            id: Uuid::new_v4(),
            transaction_key: vec!["willys".to_string(), "coop".to_string()],
            item_key: Vec::new(),
            always_apply: true,
            tag_id: None,
        };
        assert!(!rule.matches_transaction(&tx));

        // An empty key never matches.
        let rule = MatchRule {
            id: Uuid::new_v4(),
            transaction_key: Vec::new(),
            item_key: Vec::new(),
            always_apply: true,
            tag_id: None,
        };
        assert!(!rule.matches_transaction(&tx));
    }
}

impl MatchRule {
    /// Checks against an already-tokenized description. Use this (over
    /// [`Self::matches_transaction`]) whenever the same transaction is being
    /// checked against more than one rule — tokenizing is not free, and
    /// re-running it per rule turns an O(transactions) scan into
    /// O(transactions × rules). See `Budget::rule_matches`.
    pub fn matches_tokens(&self, tokens: &HashSet<String>) -> bool {
        if self.transaction_key.is_empty() {
            return false;
        }
        self.transaction_key.iter().all(|token| tokens.contains(token))
    }

    pub fn matches_transaction(&self, transaction: &BankTransaction) -> bool {
        let tokens: HashSet<String> = tokenize_description(&transaction.description).into_iter().collect();
        self.matches_tokens(&tokens)
    }

    pub fn matches_actual(&self, actual: &ActualItem) -> bool {
        let tokenized_item_name = tokenize_description(&actual.item_name);
        self.item_key == tokenized_item_name
    }

    pub fn matches_item(&self, item: &BudgetItem) -> bool {
        let tokenized_item_name = tokenize_description(&item.name);
        self.item_key == tokenized_item_name
    }

    pub fn create_rule_for_transaction_and_item(
        transaction: &BankTransaction,
        item: &ActualItem,
    ) -> MatchRule {
        let transaction_key = Self::create_transaction_key(transaction);
        MatchRule {
            id: Uuid::new_v4(),
            transaction_key,
            item_key: Self::create_item_key(item),
            always_apply: true,
            tag_id: None,
        }
    }

    pub fn create_rule_for_transaction_and_tag(
        transaction: &BankTransaction,
        tag_id: Uuid,
    ) -> MatchRule {
        MatchRule {
            id: Uuid::new_v4(),
            transaction_key: Self::create_transaction_key(transaction),
            item_key: Vec::new(),
            always_apply: true,
            tag_id: Some(tag_id),
        }
    }

    pub fn create_item_key(item: &ActualItem) -> Vec<String> {
        tokenize_description(&item.item_name)
    }

    pub fn create_transaction_key(transaction: &BankTransaction) -> Vec<String> {
        tokenize_description(&transaction.description)
    }
}
