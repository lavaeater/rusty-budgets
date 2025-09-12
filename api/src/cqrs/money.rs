use std::cmp::Ordering;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::{Add, Mul, Sub};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    EUR,
    USD,
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

#[derive(Debug, Clone, Copy, Eq, Serialize, Deserialize)]
pub struct Money {
    cents: i64, // stored in minor units (cents/öre)
    currency: Currency,
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
        Money::new_dollars(self.cents + rhs.cents, self.currency)
    }
}

impl Mul for Money {
    type Output = Money;
    fn mul(self, rhs: Self) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new_dollars(self.cents * rhs.cents, self.currency)        
    }
}

impl Sub for Money {
    type Output = Money;
    fn sub(self, rhs: Money) -> Self::Output {
        assert_eq!(self.currency, rhs.currency, "Currency mismatch");
        Money::new_dollars(self.cents - rhs.cents, self.currency)
    }
}

// Pretty-printing
impl Display for Money {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.currency {
            Currency::SEK => {
                write!(
                    f,
                    "{}:{:02} {}",
                    self.cents / 100,
                    self.cents % 100,
                    self.currency
                )
            }
            _ => {
                write!(
                    f,
                    " {}{}.{:02}",
                    self.currency,
                    self.cents / 100,
                    self.cents % 100,
                )
            }
        }
    }
}