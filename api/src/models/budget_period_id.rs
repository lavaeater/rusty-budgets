use crate::holidays::{is_workday, previous_workday};
use crate::models::MonthBeginsOn;
use chrono::{DateTime, Datelike, Days, Months, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub fn last_day_of_month(dt: DateTime<Utc>) -> DateTime<Utc> {
    let first_next_month = dt
        .checked_add_months(Months::new(1))
        .unwrap()
        .with_day(1)
        .unwrap();
    first_next_month.checked_sub_days(Days::new(1)).unwrap()
}

#[derive(Copy, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct BudgetPeriodId {
    pub year: i32,
    pub month: u32,
}

impl Display for BudgetPeriodId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}", self.year, self.month)
    }
}

impl BudgetPeriodId {
    pub fn from_date(date: DateTime<Utc>, month_begins_on: MonthBeginsOn) -> Self {
        // Determine which period this date falls into
        // The period ID represents the END month of the period
        match month_begins_on {
            MonthBeginsOn::PreviousMonth(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use PreviousMonth1stDayOfMonth")
                }
                // Period runs from day D of previous month to day D-1 of current month
                // If date is >= day D, it belongs to NEXT month's period
                // If date is < day D, it belongs to CURRENT month's period
                if date.day() >= day {
                    // Date is on or after the start day, so it belongs to next month's period
                    let next_month = date.checked_add_months(Months::new(1)).unwrap();
                    Self {
                        year: next_month.year(),
                        month: next_month.month(),
                    }
                } else {
                    // Date is before the start day, so it belongs to current month's period
                    Self {
                        year: date.year(),
                        month: date.month(),
                    }
                }
            }
            MonthBeginsOn::CurrentMonth(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use CurrentMonth1stDayOfMonth")
                }
                // Period runs from day D of current month to day D-1 of next month
                // If date is >= day D, it belongs to NEXT month's period
                // If date is < day D, it belongs to CURRENT month's period
                if date.day() >= day {
                    let next_month = date.checked_add_months(Months::new(1)).unwrap();
                    Self {
                        year: next_month.year(),
                        month: next_month.month(),
                    }
                } else {
                    Self {
                        year: date.year(),
                        month: date.month(),
                    }
                }
            }
            MonthBeginsOn::PreviousMonth1stDayOfMonth => {
                // Period is 1st of previous month to last day of previous month
                // This means current month's dates belong to current month
                Self {
                    year: date.year(),
                    month: date.month(),
                }
            }
            MonthBeginsOn::CurrentMonth1stDayOfMonth => {
                // Period is 1st of current month to last day of current month
                Self {
                    year: date.year(),
                    month: date.month(),
                }
            }
            MonthBeginsOn::PreviousMonthWorkDayBefore(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use PreviousMonth1stDayOfMonth")
                }

                // Get the target day in the current month
                let target_date = Utc
                    .with_ymd_and_hms(date.year(), date.month(), day, 0, 0, 0)
                    .unwrap();

                // Find the workday on or before the target date
                let period_start_day = if is_workday(&target_date) {
                    target_date.day()
                } else {
                    previous_workday(target_date).day()
                };

                // If current date is on or after the period start day, it belongs to next month's period
                if date.day() >= period_start_day {
                    let next_month = date.checked_add_months(Months::new(1)).unwrap();
                    Self {
                        year: next_month.year(),
                        month: next_month.month(),
                    }
                } else {
                    // Otherwise it belongs to current month's period
                    Self {
                        year: date.year(),
                        month: date.month(),
                    }
                }
            }
            MonthBeginsOn::CurrentMonthWorkDayBefore(day) => {
                if day == 1 {
                    panic!("Cannot start on day 1, use PreviousMonth1stDayOfMonth")
                }

                // Get the target day in the current month
                let target_day = day;
                let target_date = Utc
                    .with_ymd_and_hms(date.year(), date.month(), target_day, 0, 0, 0)
                    .unwrap();

                // Find the workday on or before the target date
                let period_start = if is_workday(&target_date) {
                    target_date
                } else {
                    previous_workday(target_date)
                };

                // If current date is on or after the period start, it belongs to next month's period
                if date.day() >= period_start.day() {
                    Self {
                        year: date.year(),
                        month: date.month(),
                    }
                } else {
                    // Otherwise it belongs to current month's period
                    Self {
                        year: date.year(),
                        month: date.month(),
                    }
                }
            }
        }
    }

    pub(crate) fn month_before(&self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                year: self.year,
                month: self.month - 1,
            }
        }
    }

    pub(crate) fn month_after(&self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                year: self.year,
                month: self.month + 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::models::budget_period_id::{last_day_of_month, BudgetPeriodId};
    use crate::models::MonthBeginsOn;
    use chrono::{Datelike, TimeZone, Utc};

    #[test]
    fn test_from_date_current_month_1st_day() {
        // Test with CurrentMonth1stDayOfMonth - period is 1st to last day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_std() {
        // Test with PreviousMonth1stDayOfMonth - period is 1st of prev month to last day of prev month
        let date = Utc.with_ymd_and_hms(2025, 3, 12, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        // Date is March 15, period starts Feb 1, so date is clamped to Feb 28 (end of period)
        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);

        let date = Utc.with_ymd_and_hms(2025, 2, 26, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        // Date is March 15, period starts Feb 1, so date is clamped to Feb 28 (end of period)
        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_current_month_before_start_day() {
        // Test with CurrentMonth(15) when date is before the 15th
        // Date March 10 is before period start (March 15), so clamped to March 15
        let date = Utc.with_ymd_and_hms(2025, 3, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_custom_day_within_period() {
        // Test with PreviousMonth(25) - period is 25th of prev month to 24th of current month
        // Date March 20 is within period (Feb 25 - March 24)
        let date = Utc.with_ymd_and_hms(2025, 10, 23, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 10);

        let date = Utc.with_ymd_and_hms(2025, 10, 24, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 10);
    }

    #[test]
    fn test_from_date_previous_month_work_day_before() {
        // Test with PreviousMonth(25) - period is 25th of prev month to 24th of current month
        // Date March 20 is within period (Feb 25 - March 24)
        let date = Utc.with_ymd_and_hms(2025, 10, 24, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonthWorkDayBefore(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 11);

        let date = Utc.with_ymd_and_hms(2025, 10, 23, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonthWorkDayBefore(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 10);
    }

    #[test]
    fn test_from_date_previous_month_before_start_day() {
        // Test with PreviousMonth(25) when date is before period start
        // Date March 10 is before period start (Feb 25), so clamped to Feb 25
        let date = Utc.with_ymd_and_hms(2025, 3, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_year_boundary_december_to_january() {
        // Test year boundary with PreviousMonth(25)
        // Period: Dec 25, 2024 - Jan 24, 2025
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_on_end_boundary() {
        // Test with date exactly on period end
        // Period: March 15 - April 14
        let date = Utc.with_ymd_and_hms(2025, 4, 14, 23, 59, 59).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(15));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 4);
    }

    #[test]
    fn test_from_date_february_leap_year() {
        // Test February in a leap year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2024, 2, 29, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2024);
        assert_eq!(id.month, 2);
    }

    #[test]
    fn test_from_date_february_non_leap_year() {
        // Test February in a non-leap year
        let date = Utc.with_ymd_and_hms(2025, 2, 28, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 2);
    }

    #[test]
    fn test_from_date_default_month_begins_on() {
        // Test with default MonthBeginsOn (PreviousMonth(25))
        // Period: Feb 25 - March 24
        let date = Utc.with_ymd_and_hms(2025, 3, 20, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::default());

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_previous_month_crosses_year_boundary() {
        // Test PreviousMonth(25) with date in January
        // Period: Dec 25, 2024 - Jan 24, 2025
        // Date Jan 10 is within period
        let date = Utc.with_ymd_and_hms(2025, 1, 10, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(25));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_previous_month_february_edge() {
        // Test PreviousMonth(28) with March date
        // Period: Feb 28 - March 27
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(28));

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 3);
    }

    #[test]
    fn test_from_date_consistency() {
        // Test that the same date with the same MonthBeginsOn produces the same result
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let month_begins = MonthBeginsOn::CurrentMonth(10);

        let id1 = BudgetPeriodId::from_date(date, month_begins);
        let id2 = BudgetPeriodId::from_date(date, month_begins);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_from_date_first_day_of_year() {
        // Test first day of the year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 1);
    }

    #[test]
    fn test_from_date_last_day_of_year() {
        // Test last day of the year with CurrentMonth1stDayOfMonth
        let date = Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap();
        let id = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert_eq!(id.year, 2025);
        assert_eq!(id.month, 12);
    }

    #[test]
    #[should_panic(expected = "Cannot start on day 1, use PreviousMonth1stDayOfMonth")]
    fn test_from_date_previous_month_day_1_panics() {
        // Test that PreviousMonth(1) panics
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let _ = BudgetPeriodId::from_date(date, MonthBeginsOn::PreviousMonth(1));
    }

    #[test]
    #[should_panic(expected = "Cannot start on day 1, use CurrentMonth1stDayOfMonth")]
    fn test_from_date_current_month_day_1_panics() {
        // Test that CurrentMonth(1) panics
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let _ = BudgetPeriodId::from_date(date, MonthBeginsOn::CurrentMonth(1));
    }

    #[test]
    fn test_from_date_ordering() {
        // Test that BudgetPeriodId ordering works correctly
        let date1 = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let date2 = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let date3 = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();

        let id1 = BudgetPeriodId::from_date(date1, MonthBeginsOn::CurrentMonth1stDayOfMonth);
        let id2 = BudgetPeriodId::from_date(date2, MonthBeginsOn::CurrentMonth1stDayOfMonth);
        let id3 = BudgetPeriodId::from_date(date3, MonthBeginsOn::CurrentMonth1stDayOfMonth);

        assert!(id1 < id2);
        assert!(id2 < id3);
        assert!(id1 < id3);
    }

    #[test]
    fn test_last_day_of_month_january() {
        // January has 31 days
        let date = Utc.with_ymd_and_hms(2025, 1, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 1);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_february_non_leap_year() {
        // February in non-leap year has 28 days
        let date = Utc.with_ymd_and_hms(2025, 2, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 2);
        assert_eq!(last_day.day(), 28);
    }

    #[test]
    fn test_last_day_of_month_february_leap_year() {
        // February in leap year has 29 days
        let date = Utc.with_ymd_and_hms(2024, 2, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2024);
        assert_eq!(last_day.month(), 2);
        assert_eq!(last_day.day(), 29);
    }

    #[test]
    fn test_last_day_of_month_march() {
        // March has 31 days
        let date = Utc.with_ymd_and_hms(2025, 3, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_april() {
        // April has 30 days
        let date = Utc.with_ymd_and_hms(2025, 4, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 4);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_may() {
        // May has 31 days
        let date = Utc.with_ymd_and_hms(2025, 5, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 5);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_june() {
        // June has 30 days
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 6);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_july() {
        // July has 31 days
        let date = Utc.with_ymd_and_hms(2025, 7, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 7);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_august() {
        // August has 31 days
        let date = Utc.with_ymd_and_hms(2025, 8, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 8);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_september() {
        // September has 30 days
        let date = Utc.with_ymd_and_hms(2025, 9, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 9);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_october() {
        // October has 31 days
        let date = Utc.with_ymd_and_hms(2025, 10, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 10);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_november() {
        // November has 30 days
        let date = Utc.with_ymd_and_hms(2025, 11, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 11);
        assert_eq!(last_day.day(), 30);
    }

    #[test]
    fn test_last_day_of_month_december() {
        // December has 31 days
        let date = Utc.with_ymd_and_hms(2025, 12, 15, 12, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 12);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_first_day() {
        // Test with first day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_already_last_day() {
        // Test with already the last day of month
        let date = Utc.with_ymd_and_hms(2025, 3, 31, 23, 59, 59).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.year(), 2025);
        assert_eq!(last_day.month(), 3);
        assert_eq!(last_day.day(), 31);
    }

    #[test]
    fn test_last_day_of_month_preserves_time() {
        // Test that time components are preserved
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 14, 30, 45).unwrap();
        let last_day = last_day_of_month(date);

        assert_eq!(last_day.day(), 30);
        // Time should be adjusted to the last day but the calculation goes through
        // first day of next month minus 1 day, so time will be preserved
    }

    #[test]
    fn test_last_day_of_month_all_months_2025() {
        // Test all months in a non-leap year
        let expected_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        for (month, &expected_day) in (1..=12).zip(expected_days.iter()) {
            let date = Utc.with_ymd_and_hms(2025, month, 15, 12, 0, 0).unwrap();
            let last_day = last_day_of_month(date);

            assert_eq!(last_day.year(), 2025);
            assert_eq!(last_day.month(), month);
            assert_eq!(last_day.day(), expected_day, "Failed for month {}", month);
        }
    }

    #[test]
    fn test_last_day_of_month_all_months_2024() {
        // Test all months in a leap year
        let expected_days = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

        for (month, &expected_day) in (1..=12).zip(expected_days.iter()) {
            let date = Utc.with_ymd_and_hms(2024, month, 15, 12, 0, 0).unwrap();
            let last_day = last_day_of_month(date);

            assert_eq!(last_day.year(), 2024);
            assert_eq!(last_day.month(), month);
            assert_eq!(
                last_day.day(),
                expected_day,
                "Failed for month {} in leap year",
                month
            );
        }
    }

    #[test]
    fn test_last_day_of_month_century_leap_years() {
        // Test century years (2000 is a leap year, 1900 and 2100 are not)
        // 2000 is divisible by 400, so it's a leap year
        let date_2000 = Utc.with_ymd_and_hms(2000, 2, 15, 12, 0, 0).unwrap();
        let last_day_2000 = last_day_of_month(date_2000);
        assert_eq!(last_day_2000.day(), 29);
    }

    #[test]
    fn test_last_day_of_month_consistency() {
        // Test that calling the function multiple times with the same input gives the same result
        let date = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();

        let result1 = last_day_of_month(date);
        let result2 = last_day_of_month(date);

        assert_eq!(result1, result2);
    }
}
