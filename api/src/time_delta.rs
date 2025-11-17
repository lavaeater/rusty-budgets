use chrono::TimeDelta;

pub trait TimeDeltaExt {
    fn seconds(self) -> TimeDelta;
    fn minutes(self) -> TimeDelta;
    fn hours(self) -> TimeDelta;
    fn millis(self) -> TimeDelta;
    fn micros(self) -> TimeDelta;
    fn nanos(self) -> TimeDelta;
    fn days(self) -> TimeDelta;
}

impl TimeDeltaExt for u64 {
    fn seconds(self) -> TimeDelta {
        TimeDelta::seconds(self as i64)
    }
    fn minutes(self) -> TimeDelta {
        TimeDelta::minutes(self as i64)
    }
    fn hours(self) -> TimeDelta {
        TimeDelta::hours(self as i64)
    }
    fn millis(self) -> TimeDelta {
        TimeDelta::milliseconds(self as i64)
    }
    fn micros(self) -> TimeDelta {
        TimeDelta::microseconds(self as i64)
    }
    fn nanos(self) -> TimeDelta {
        TimeDelta::nanoseconds(self as i64)
    }

    fn days(self) -> TimeDelta {
        TimeDelta::days(self as i64)
    }
}

impl TimeDeltaExt for i64 {
    fn seconds(self) -> TimeDelta {
        TimeDelta::seconds(self)
    }
    fn minutes(self) -> TimeDelta {
        TimeDelta::minutes(self)
    }
    fn hours(self) -> TimeDelta {
        TimeDelta::hours(self)
    }
    fn millis(self) -> TimeDelta {
        TimeDelta::milliseconds(self)
    }
    fn micros(self) -> TimeDelta {
        TimeDelta::microseconds(self)
    }
    fn nanos(self) -> TimeDelta {
        TimeDelta::nanoseconds(self)
    }

    fn days(self) -> TimeDelta {
        TimeDelta::days(self)
    }
}

impl TimeDeltaExt for u32 {
    fn seconds(self) -> TimeDelta {
        TimeDelta::seconds(self as i64)
    }
    fn minutes(self) -> TimeDelta {
        TimeDelta::minutes(self as i64)
    }
    fn hours(self) -> TimeDelta {
        TimeDelta::hours(self as i64)
    }
    fn millis(self) -> TimeDelta {
        TimeDelta::milliseconds(self as i64)
    }
    fn micros(self) -> TimeDelta {
        TimeDelta::microseconds(self as i64)
    }
    fn nanos(self) -> TimeDelta {
        TimeDelta::nanoseconds(self as i64)
    }

    fn days(self) -> TimeDelta {
        TimeDelta::days(self as i64)
    }
}

impl TimeDeltaExt for i32 {
    fn seconds(self) -> TimeDelta {
        TimeDelta::seconds(self as i64)
    }
    fn minutes(self) -> TimeDelta {
        TimeDelta::minutes(self as i64)
    }
    fn hours(self) -> TimeDelta {
        TimeDelta::hours(self as i64)
    }
    fn millis(self) -> TimeDelta {
        TimeDelta::milliseconds(self as i64)
    }
    fn micros(self) -> TimeDelta {
        TimeDelta::microseconds(self as i64)
    }
    fn nanos(self) -> TimeDelta {
        TimeDelta::nanoseconds(self as i64)
    }

    fn days(self) -> TimeDelta {
        TimeDelta::days(self as i64)
    }
}