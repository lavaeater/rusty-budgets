use chrono::{DateTime, Utc, TimeDelta, Datelike, Weekday, TimeZone};
use crate::time_delta::TimeDeltaExt;
/// Checks if a date is a weekend (Saturday or Sunday)
fn is_weekend(date: &DateTime<Utc>) -> bool {
    date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun
}

/// Checks if a date is a Swedish public holiday
fn is_swedish_holiday(date: &DateTime<Utc>) -> bool {
    let year = date.year();
    let month = date.month();
    let day = date.day();
    let easter_sunday = calculate_easter_sunday(year);

    let fixed_date = 
    // Fixed date holidays
    match (month, day) {
        (1, 1) => true,   // New Year's Day
        (1, 5) => true,   // Twelfth Night
        (1, 6) => true,   // Epiphany
        (5, 1) => true,   // May Day
        (6, 6) => true,   // National Day of Sweden
        (12, 24) => true, // Christmas Eve
        (12, 25) => true, // Christmas Day
        (12, 26) => true, // Second Day of Christmas
        (12, 31) => true, // New Year's Eve
        _ => false,
    };
    let moving_holidays = {
        // Moving holidays (based on Easter)
        let good_friday = easter_sunday - 2.days();
        let easter_monday = easter_sunday + 1.days();
        let ascension = easter_sunday + 39.days();
        let pentecost = easter_sunday + 49.days();
        let midsummer_eve = calculate_midsummer_eve(year);
        let all_saints_day = calculate_all_saints_day(year);

        let date_naive = date.date_naive();
        date_naive == good_friday.date_naive()
            || date_naive == easter_sunday.date_naive()
            || date_naive == easter_monday.date_naive()
            || date_naive == ascension.date_naive()
            || date_naive == pentecost.date_naive()
            || date_naive == midsummer_eve.date_naive()
            || date_naive == all_saints_day.date_naive()
    };
    fixed_date || moving_holidays
}

/// Checks if a date is a workday (not weekend and not a holiday)
pub fn is_workday(date: &DateTime<Utc>) -> bool {
    !is_weekend(date) && !is_swedish_holiday(date)
}
/// Finds the previous workday before the given date
pub fn previous_workday(mut date: DateTime<Utc>) -> DateTime<Utc> {
    loop {
        date -= 1.days();
        if is_workday(&date) {
            return date;
        }
    }
}

/// Finds the next workday after the given date
pub fn next_workday(mut date: DateTime<Utc>) -> DateTime<Utc> {
    loop {
        date += 1.days();
        if is_workday(&date) {
            return date;
        }
    }
}

/// Calculates Easter Sunday for a given year using the Meeus/Jones/Butcher algorithm
fn calculate_easter_sunday(year: i32) -> DateTime<Utc> {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;

    Utc.with_ymd_and_hms(year, month as u32, day as u32, 0, 0, 0)
        .unwrap()
}

/// Calculates Midsummer Eve (Friday between June 19-25)
fn calculate_midsummer_eve(year: i32) -> DateTime<Utc> {
    // Start from June 19 and find the next Friday
    let mut date = Utc.with_ymd_and_hms(year, 6, 19, 0, 0, 0).unwrap();
    while date.weekday() != chrono::Weekday::Fri {
        date += 1.days();
    }
    date
}

/// Calculates All Saints' Day (Saturday between October 31 and November 6)
fn calculate_all_saints_day(year: i32) -> DateTime<Utc> {
    // Start from October 31 and find the next Saturday
    let mut date = Utc.with_ymd_and_hms(year, 10, 31, 0, 0, 0).unwrap();
    while date.weekday() != chrono::Weekday::Sat {
        date += 1.days();
    }
    date
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_weekend_detection() {
        // A known Saturday
        let saturday = Utc.with_ymd_and_hms(2023, 4, 1, 0, 0, 0).unwrap();
        assert!(is_weekend(&saturday));

        // A known Sunday
        let sunday = Utc.with_ymd_and_hms(2023, 4, 2, 0, 0, 0).unwrap();
        assert!(is_weekend(&sunday));

        // A known Monday
        let monday = Utc.with_ymd_and_hms(2023, 4, 3, 0, 0, 0).unwrap();
        assert!(!is_weekend(&monday));
    }

    #[test]
    fn test_swedish_holidays() {
        // New Year's Day
        let new_years = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        assert!(is_swedish_holiday(&new_years));

        // Midsummer Eve 2023 (June 23)
        let midsummer_eve = Utc.with_ymd_and_hms(2023, 6, 23, 0, 0, 0).unwrap();
        assert!(is_swedish_holiday(&midsummer_eve));

        // Christmas Day
        let christmas = Utc.with_ymd_and_hms(2023, 12, 25, 0, 0, 0).unwrap();
        assert!(is_swedish_holiday(&christmas));

        // A random non-holiday
        let regular_day = Utc.with_ymd_and_hms(2023, 4, 15, 0, 0, 0).unwrap();
        assert!(!is_swedish_holiday(&regular_day));
    }

    #[test]
    fn test_workday_calculation() {
        // A regular workday (Monday, April 3, 2023)
        let workday = Utc.with_ymd_and_hms(2023, 4, 3, 0, 0, 0).unwrap();
        assert!(is_workday(&workday));

        // A weekend (Saturday, April 1, 2023)
        let weekend = Utc.with_ymd_and_hms(2023, 4, 1, 0, 0, 0).unwrap();
        assert!(!is_workday(&weekend));

        // A holiday (New Year's Day 2023)
        let holiday = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
        assert!(!is_workday(&holiday));
    }

    #[test]
    fn test_previous_workday() {
        // Friday before a weekend
        let friday = Utc.with_ymd_and_hms(2023, 4, 7, 0, 0, 0).unwrap();
        let thursday = Utc.with_ymd_and_hms(2023, 4, 6, 0, 0, 0).unwrap();
        assert_eq!(previous_workday(friday), thursday);

        // Monday after a weekend
        let monday = Utc.with_ymd_and_hms(2025, 11, 10, 0, 0, 0).unwrap();
        let friday_before = Utc.with_ymd_and_hms(2025, 11, 7, 0, 0, 0).unwrap();
        assert_eq!(previous_workday(monday), friday_before);
    }

    #[test]
    fn test_easter_calculation() {
        // Known Easter dates
        assert_eq!(
            calculate_easter_sunday(2023).date_naive(),
            chrono::NaiveDate::from_ymd_opt(2023, 4, 9).unwrap()
        );
        assert_eq!(
            calculate_easter_sunday(2024).date_naive(),
            chrono::NaiveDate::from_ymd_opt(2024, 3, 31).unwrap()
        );
    }
}

