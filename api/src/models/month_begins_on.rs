use std::fmt::{Display, Formatter};
use std::str::FromStr;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum MonthBeginsOn {
    PreviousMonth(u32),
    PreviousMonthWorkDayBefore(u32),
    CurrentMonth(u32),
    CurrentMonthWorkDayBefore(u32),
    PreviousMonth1stDayOfMonth,
    CurrentMonth1stDayOfMonth,
}

impl Default for MonthBeginsOn {
    fn default() -> Self {
        MonthBeginsOn::PreviousMonthWorkDayBefore(25)
    }
}

impl Display for MonthBeginsOn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MonthBeginsOn::PreviousMonth(day) => write!(f, "PreviousMonth({})", day),
            MonthBeginsOn::PreviousMonthWorkDayBefore(day) => write!(f, "PreviousMonthWorkDayBefore({})", day),
            MonthBeginsOn::CurrentMonth(day) => write!(f, "CurrentMonth({})", day),
            MonthBeginsOn::CurrentMonthWorkDayBefore(day) => write!(f, "CurrentMonthWorkDayBefore({})", day),
            MonthBeginsOn::PreviousMonth1stDayOfMonth => write!(f, "PreviousMonth1stDayOfMonth"),
            MonthBeginsOn::CurrentMonth1stDayOfMonth => write!(f, "CurrentMonth1stDayOfMonth"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default() {
        let default = MonthBeginsOn::default();
        assert_eq!(default, MonthBeginsOn::PreviousMonthWorkDayBefore(25));
    }

    #[test]
    fn test_serde_round_trip() {
        let variants = vec![
            MonthBeginsOn::PreviousMonth(25),
            MonthBeginsOn::PreviousMonthWorkDayBefore(25),
            MonthBeginsOn::CurrentMonth(15),
            MonthBeginsOn::CurrentMonthWorkDayBefore(10),
            MonthBeginsOn::PreviousMonth1stDayOfMonth,
            MonthBeginsOn::CurrentMonth1stDayOfMonth,
        ];

        for variant in variants {
            let serialized = serde_json::to_string(&variant).unwrap();
            let deserialized: MonthBeginsOn = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, variant, "Serde round trip failed for variant: {:?}", variant);
        }
    }
}