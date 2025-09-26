use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::iter::Sum;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    EUR,
    USD,
    #[default]
    SEK,
    // extend as needed
}

impl Display for Currency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Currency::EUR => f.write_str("€"),
            Currency::USD => f.write_str("$"),
            Currency::SEK => f.write_str("kr"),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, Eq, Serialize, Deserialize)]
pub struct Money {
    cents: i64, // stored in minor units (cents/öre)
    currency: Currency,
}

impl Neg for Money {
    type Output = Money;
    fn neg(self) -> Self::Output {
        Money::new_cents(-self.cents, self.currency)
    }
}

impl PartialOrd for Money {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.currency != other.currency {
            None
        } else {
            Some(self.cents.cmp(&other.cents))
        }
    }
}

impl PartialEq for Money {
    fn eq(&self, other: &Self) -> bool {
        self.cents == other.cents && self.currency == other.currency
    }
}

impl Hash for Money {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.cents.hash(state);
        self.currency.hash(state);
    }
}

impl Money {
    pub fn new_dollars(dollars: i64, currency: Currency) -> Self {
        Self { cents: dollars * 100, currency }
    }
    pub fn new_cents(cents: i64, currency: Currency) -> Self {
        Self { cents, currency }
    }

    pub fn amount_in_cents(&self) -> i64 {
        self.cents
    }
    
    pub fn amount_in_dollars(&self) -> i64 { 
        self.cents / 100
    }
    

    pub fn currency(&self) -> Currency {
        self.currency
    }
}

impl Add for Money {
    type Output = Money;
    fn add(self, rhs: Money) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new_cents(self.cents + rhs.cents, self.currency)
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Money) {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        self.cents += rhs.cents;
    }
}

impl Mul for Money {
    type Output = Money;
    fn mul(self, rhs: Self) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new_cents(self.cents * rhs.cents / 100, self.currency)        
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new_cents(self.cents - rhs.cents, self.currency)
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Money) {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        self.cents -= rhs.cents;
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Money::default(), |a, b| a + b)
    }
}

// Pretty-printing
impl Display for Money {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.currency {
            Currency::SEK => {
                write!(
                    f,
                    "{} {}",
                    self.amount_in_dollars(),
                    self.currency
                )
            }
            _ => {
                write!(
                    f,
                    " {}{}",
                    self.currency,
                    self.amount_in_dollars(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_currency_display() {
        assert_eq!(Currency::EUR.to_string(), "€");
        assert_eq!(Currency::USD.to_string(), "$");
        assert_eq!(Currency::SEK.to_string(), "kr");
    }

    #[test]
    fn test_money_creation() {
        let money_dollars = Money::new_dollars(100, Currency::USD);
        assert_eq!(money_dollars.amount_in_cents(), 10000);
        assert_eq!(money_dollars.amount_in_dollars(), 100);
        assert_eq!(money_dollars.currency(), Currency::USD);

        let money_cents = Money::new_cents(12345, Currency::EUR);
        assert_eq!(money_cents.amount_in_cents(), 12345);
        assert_eq!(money_cents.amount_in_dollars(), 123);
        assert_eq!(money_cents.currency(), Currency::EUR);
    }

    #[test]
    fn test_money_display_sek() {
        let money = Money::new_dollars(100, Currency::SEK);
        assert_eq!(money.to_string(), "100 kr");

        let money_zero = Money::new_dollars(0, Currency::SEK);
        assert_eq!(money_zero.to_string(), "0 kr");

        let money_negative = Money::new_dollars(-50, Currency::SEK);
        assert_eq!(money_negative.to_string(), "-50 kr");

        let money_large = Money::new_dollars(1234567, Currency::SEK);
        assert_eq!(money_large.to_string(), "1234567 kr");
    }

    #[test]
    fn test_money_display_usd() {
        let money = Money::new_dollars(100, Currency::USD);
        assert_eq!(money.to_string(), " $100");

        let money_zero = Money::new_dollars(0, Currency::USD);
        assert_eq!(money_zero.to_string(), " $0");

        let money_negative = Money::new_dollars(-50, Currency::USD);
        assert_eq!(money_negative.to_string(), " $-50");

        let money_large = Money::new_dollars(1234567, Currency::USD);
        assert_eq!(money_large.to_string(), " $1234567");
    }

    #[test]
    fn test_money_display_eur() {
        let money = Money::new_dollars(100, Currency::EUR);
        assert_eq!(money.to_string(), " €100");

        let money_zero = Money::new_dollars(0, Currency::EUR);
        assert_eq!(money_zero.to_string(), " €0");

        let money_negative = Money::new_dollars(-50, Currency::EUR);
        assert_eq!(money_negative.to_string(), " €-50");

        let money_large = Money::new_dollars(1234567, Currency::EUR);
        assert_eq!(money_large.to_string(), " €1234567");
    }

    #[test]
    fn test_money_display_with_cents() {
        // Test with cents that don't divide evenly into dollars
        let money_cents = Money::new_cents(12345, Currency::USD);
        assert_eq!(money_cents.to_string(), " $123");

        let money_cents_sek = Money::new_cents(12345, Currency::SEK);
        assert_eq!(money_cents_sek.to_string(), "123 kr");

        let money_cents_eur = Money::new_cents(9876, Currency::EUR);
        assert_eq!(money_cents_eur.to_string(), " €98");
    }

    #[test]
    fn test_money_arithmetic() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(50, Currency::USD);

        let sum = money1 + money2;
        assert_eq!(sum.amount_in_dollars(), 150);
        assert_eq!(sum.currency(), Currency::USD);

        let diff = money1 - money2;
        assert_eq!(diff.amount_in_dollars(), 50);
        assert_eq!(diff.currency(), Currency::USD);

        let product = money1 * money2;
        let some_product = money1.amount_in_dollars() * money2.amount_in_dollars();
        assert_eq!(some_product, 5000);
        assert_eq!(product.amount_in_dollars(), 5000);
        assert_eq!(product.currency(), Currency::USD);
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn test_money_arithmetic_currency_mismatch_add() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(50, Currency::EUR);
        let _ = money1 + money2;
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn test_money_arithmetic_currency_mismatch_sub() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(50, Currency::EUR);
        let _ = money1 - money2;
    }

    #[test]
    #[should_panic(expected = "Currency mismatch")]
    fn test_money_arithmetic_currency_mismatch_mul() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(50, Currency::EUR);
        let _ = money1 * money2;
    }

    #[test]
    fn test_money_equality() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(100, Currency::USD);
        let money3 = Money::new_dollars(100, Currency::EUR);
        let money4 = Money::new_dollars(50, Currency::USD);

        assert_eq!(money1, money2);
        assert_ne!(money1, money3); // Different currency
        assert_ne!(money1, money4); // Different amount
    }

    #[test]
    fn test_money_partial_ord() {
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(50, Currency::USD);
        let money3 = Money::new_dollars(100, Currency::EUR);

        assert!(money1 > money2);
        assert!(money2 < money1);
        assert_eq!(money1.partial_cmp(&money2), Some(Ordering::Greater));
        assert_eq!(money2.partial_cmp(&money1), Some(Ordering::Less));
        
        // Different currencies should return None
        assert_eq!(money1.partial_cmp(&money3), None);
    }

    #[test]
    fn test_money_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(100, Currency::USD);
        let money3 = Money::new_dollars(50, Currency::USD);

        let mut hasher1 = DefaultHasher::new();
        let mut hasher2 = DefaultHasher::new();
        let mut hasher3 = DefaultHasher::new();

        money1.hash(&mut hasher1);
        money2.hash(&mut hasher2);
        money3.hash(&mut hasher3);

        let hash1 = hasher1.finish();
        let hash2 = hasher2.finish();
        let hash3 = hasher3.finish();

        // Equal objects should have equal hashes
        assert_eq!(hash1, hash2);
        // Different objects should have different hashes (not guaranteed but very likely)
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_money_hash_set_behavior() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let money1 = Money::new_dollars(100, Currency::USD);
        let money2 = Money::new_dollars(100, Currency::USD);
        let money3 = Money::new_dollars(50, Currency::USD);

        assert!(set.insert(money1));
        assert!(!set.insert(money2)); // Should not insert duplicate
        assert!(set.insert(money3)); // Should insert different amount

        assert_eq!(set.len(), 2);
    }

    #[test]
    fn test_currency_traits() {
        // Test that Currency implements the expected traits
        let eur = Currency::EUR;
        let eur_clone = eur.clone();
        
        assert_eq!(eur, eur_clone);
        assert_eq!(eur, Currency::EUR);
        assert_ne!(eur, Currency::USD);

        // Test Debug formatting
        let debug_str = format!("{:?}", Currency::SEK);
        assert_eq!(debug_str, "SEK");
    }

    #[test]
    fn test_money_edge_cases() {
        // Test with zero
        let zero_usd = Money::new_dollars(0, Currency::USD);
        let zero_eur = Money::new_dollars(0, Currency::EUR);
        assert_eq!(zero_usd.to_string(), " $0");
        assert_eq!(zero_eur.to_string(), " €0");

        // Test with negative values
        let negative_sek = Money::new_dollars(-100, Currency::SEK);
        assert_eq!(negative_sek.to_string(), "-100 kr");
        assert_eq!(negative_sek.amount_in_dollars(), -100);

        // Test with very large values
        let large_money = Money::new_dollars(i64::MAX / 100, Currency::USD);
        assert_eq!(large_money.amount_in_cents(), (i64::MAX / 100) * 100);
    }
}