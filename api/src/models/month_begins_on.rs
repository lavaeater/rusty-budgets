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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseMonthBeginsOnError(String);

impl Display for ParseMonthBeginsOnError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to parse MonthBeginsOn: {}", self.0)
    }
}

impl std::error::Error for ParseMonthBeginsOnError {}

impl FromStr for MonthBeginsOn {
    type Err = ParseMonthBeginsOnError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Handle variants without parameters
        match s {
            "PreviousMonth1stDayOfMonth" => return Ok(MonthBeginsOn::PreviousMonth1stDayOfMonth),
            "CurrentMonth1stDayOfMonth" => return Ok(MonthBeginsOn::CurrentMonth1stDayOfMonth),
            _ => {}
        }
        
        // Handle variants with parameters: "VariantName(day)"
        if let Some(open_paren) = s.find('(') {
            if let Some(close_paren) = s.find(')') {
                let variant_name = &s[..open_paren];
                let day_str = &s[open_paren + 1..close_paren];
                let day: u32 = day_str.parse()
                    .map_err(|_| ParseMonthBeginsOnError(format!("Invalid day value: {}", day_str)))?;
                
                match variant_name {
                    "PreviousMonth" => Ok(MonthBeginsOn::PreviousMonth(day)),
                    "PreviousMonthWorkDayBefore" => Ok(MonthBeginsOn::PreviousMonthWorkDayBefore(day)),
                    "CurrentMonth" => Ok(MonthBeginsOn::CurrentMonth(day)),
                    "CurrentMonthWorkDayBefore" => Ok(MonthBeginsOn::CurrentMonthWorkDayBefore(day)),
                    _ => Err(ParseMonthBeginsOnError(format!("Unknown variant: {}", variant_name))),
                }
            } else {
                Err(ParseMonthBeginsOnError(format!("Missing closing parenthesis in: {}", s)))
            }
        } else {
            Err(ParseMonthBeginsOnError(format!("Invalid format: {}", s)))
        }
    }
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
    fn test_display_previous_month() {
        let month_begins = MonthBeginsOn::PreviousMonth(25);
        assert_eq!(month_begins.to_string(), "PreviousMonth(25)");
    }

    #[test]
    fn test_display_previous_month_workday_before() {
        let month_begins = MonthBeginsOn::PreviousMonthWorkDayBefore(25);
        assert_eq!(month_begins.to_string(), "PreviousMonthWorkDayBefore(25)");
    }

    #[test]
    fn test_display_current_month() {
        let month_begins = MonthBeginsOn::CurrentMonth(15);
        assert_eq!(month_begins.to_string(), "CurrentMonth(15)");
    }

    #[test]
    fn test_display_current_month_workday_before() {
        let month_begins = MonthBeginsOn::CurrentMonthWorkDayBefore(10);
        assert_eq!(month_begins.to_string(), "CurrentMonthWorkDayBefore(10)");
    }

    #[test]
    fn test_display_previous_month_1st_day() {
        let month_begins = MonthBeginsOn::PreviousMonth1stDayOfMonth;
        assert_eq!(month_begins.to_string(), "PreviousMonth1stDayOfMonth");
    }

    #[test]
    fn test_display_current_month_1st_day() {
        let month_begins = MonthBeginsOn::CurrentMonth1stDayOfMonth;
        assert_eq!(month_begins.to_string(), "CurrentMonth1stDayOfMonth");
    }

    #[test]
    fn test_from_str_previous_month() {
        let parsed: MonthBeginsOn = "PreviousMonth(25)".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::PreviousMonth(25));
    }

    #[test]
    fn test_from_str_previous_month_workday_before() {
        let parsed: MonthBeginsOn = "PreviousMonthWorkDayBefore(25)".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::PreviousMonthWorkDayBefore(25));
    }

    #[test]
    fn test_from_str_current_month() {
        let parsed: MonthBeginsOn = "CurrentMonth(15)".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::CurrentMonth(15));
    }

    #[test]
    fn test_from_str_current_month_workday_before() {
        let parsed: MonthBeginsOn = "CurrentMonthWorkDayBefore(10)".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::CurrentMonthWorkDayBefore(10));
    }

    #[test]
    fn test_from_str_previous_month_1st_day() {
        let parsed: MonthBeginsOn = "PreviousMonth1stDayOfMonth".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::PreviousMonth1stDayOfMonth);
    }

    #[test]
    fn test_from_str_current_month_1st_day() {
        let parsed: MonthBeginsOn = "CurrentMonth1stDayOfMonth".parse().unwrap();
        assert_eq!(parsed, MonthBeginsOn::CurrentMonth1stDayOfMonth);
    }

    #[test]
    fn test_round_trip_all_variants() {
        let variants = vec![
            MonthBeginsOn::PreviousMonth(25),
            MonthBeginsOn::PreviousMonthWorkDayBefore(25),
            MonthBeginsOn::CurrentMonth(15),
            MonthBeginsOn::CurrentMonthWorkDayBefore(10),
            MonthBeginsOn::PreviousMonth1stDayOfMonth,
            MonthBeginsOn::CurrentMonth1stDayOfMonth,
        ];

        for variant in variants {
            let string = variant.to_string();
            let parsed: MonthBeginsOn = string.parse().unwrap();
            assert_eq!(parsed, variant, "Round trip failed for variant: {}", string);
        }
    }

    #[test]
    fn test_from_str_invalid_variant() {
        let result: Result<MonthBeginsOn, _> = "InvalidVariant(25)".parse();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to parse MonthBeginsOn: Unknown variant: InvalidVariant"
        );
    }

    #[test]
    fn test_from_str_invalid_day() {
        let result: Result<MonthBeginsOn, _> = "PreviousMonth(abc)".parse();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to parse MonthBeginsOn: Invalid day value: abc"
        );
    }

    #[test]
    fn test_from_str_missing_closing_paren() {
        let result: Result<MonthBeginsOn, _> = "PreviousMonth(25".parse();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to parse MonthBeginsOn: Missing closing parenthesis in: PreviousMonth(25"
        );
    }

    #[test]
    fn test_from_str_invalid_format() {
        let result: Result<MonthBeginsOn, _> = "InvalidFormat".parse();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed to parse MonthBeginsOn: Invalid format: InvalidFormat"
        );
    }

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