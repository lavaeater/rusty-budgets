use crate::models::money::Money;
use crate::models::{MonthBeginsOn, PeriodId};
use chrono::{DateTime, Utc};
use core::fmt;
use core::fmt::{Display, Formatter};
use serde::{Deserialize, Serialize};
use std::hash::{DefaultHasher, Hash, Hasher};
use uuid::Uuid;

/// Strips everything but ASCII digits — the canonical form account numbers
/// are stored and compared in, regardless of how a bank punctuates them for
/// display (e.g. Skandiabanken's `9159-482.485-3`). Storing the punctuated
/// form let the same physical account be imported as two different
/// [`BankAccount`]s depending on which sheet cell it came from.
pub fn normalize_account_number(raw: &str) -> String {
    raw.chars().filter(char::is_ascii_digit).collect()
}

/// Formats a normalized (digits-only) Skandiabanken-style account number for
/// display: `XXXX-XXX.XXX-X` (4-digit clearing number + 3+3+1 digit account
/// number). Anything that isn't exactly 11 digits — a different bank's
/// format, or a number that hasn't been normalized — is returned unchanged.
pub fn format_account_number(digits: &str) -> String {
    if digits.len() == 11 && digits.bytes().all(|b| b.is_ascii_digit()) {
        format!(
            "{}-{}.{}-{}",
            &digits[0..4],
            &digits[4..7],
            &digits[7..10],
            &digits[10..11]
        )
    } else {
        digits.to_string()
    }
}

/// What an account is used for. Drives how transfers into/out of it are
/// interpreted — most importantly, a transfer landing in a `Savings`
/// account is always a savings contribution, never a neutral internal
/// float, so the UI shouldn't offer that resolution for it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum BankAccountType {
    #[default]
    Checking,
    Billing,
    Savings,
    Personal,
    CreditCard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BankAccount {
    pub id: Uuid,
    pub account_number: String,
    pub description: String,
    pub currency: String,
    pub balance: Money,
    #[serde(default)]
    pub account_type: BankAccountType,
}

#[derive(Debug, Clone, Serialize, Deserialize, Eq, Default)]
pub struct BankTransaction {
    pub id: Uuid,
    pub account_number: String,
    pub amount: Money,
    pub description: String,
    pub date: DateTime<Utc>,
    pub actual_id: Option<Uuid>,
    pub balance: Money,
    pub ignored: bool,
    #[serde(default)]
    pub tag_id: Option<Uuid>,
}

impl PartialEq for BankTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.amount == other.amount
            && self.description == other.description
            && self.date == other.date
    }

    // fn ne(&self, other: &Self) -> bool {
    //     self.amount != other.amount || self.description != other.description || self.date != other.date
    // }
}

impl Hash for BankTransaction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.amount.hash(state);
        self.balance.hash(state);
        self.account_number.hash(state);
        self.description.hash(state);
        self.date.hash(state);
    }
}

impl BankTransaction {
    pub fn get_hash(&self) -> u64 {
        get_transaction_hash(
            &self.amount,
            &self.balance,
            &self.account_number,
            &self.description,
            &self.date,
        )
    }
}

pub fn get_transaction_hash(
    amount: &Money,
    balance: &Money,
    account_number: &str,
    description: &str,
    date: &DateTime<Utc>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    amount.hash(&mut hasher);
    balance.hash(&mut hasher);
    account_number.hash(&mut hasher);
    description.hash(&mut hasher);
    date.hash(&mut hasher);
    hasher.finish()
}

impl Display for BankTransaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}, {}",
            self.description, self.amount, self.date, self.ignored
        )
    }
}

impl BankTransaction {
    pub fn new(
        id: Uuid,
        account_number: &str,
        amount: Money,
        balance: Money,
        description: &str,
        date: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            account_number: account_number.to_string(),
            amount,
            balance,
            description: description.to_string(),
            date,
            actual_id: None,
            ignored: false,
            tag_id: None,
        }
    }

    pub fn period_id(&self, month_begins_on: MonthBeginsOn) -> PeriodId {
        PeriodId::from_date(self.date, month_begins_on)
    }
}

#[cfg(test)]
mod normalization_tests {
    use super::*;

    #[test]
    fn normalize_strips_hyphens_and_periods() {
        assert_eq!(normalize_account_number("9159-482.485-3"), "91594824853");
        assert_eq!(normalize_account_number("91594824853"), "91594824853");
    }

    #[test]
    fn format_punctuates_an_eleven_digit_number() {
        assert_eq!(format_account_number("91594824853"), "9159-482.485-3");
    }

    #[test]
    fn format_leaves_non_eleven_digit_numbers_unchanged() {
        assert_eq!(format_account_number("1234567890"), "1234567890");
        assert_eq!(format_account_number(""), "");
    }
}
