use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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