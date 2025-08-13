use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MonthBeginsOn {
    PreviousMonth(u32),
    CurrentMonth(u32),
}

impl Default for MonthBeginsOn {
    fn default() -> Self {
        MonthBeginsOn::PreviousMonth(25)
    }
}