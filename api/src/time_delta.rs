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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_conversions() {
        assert_eq!(3u64.seconds(), TimeDelta::seconds(3));
        assert_eq!(2u64.minutes(), TimeDelta::minutes(2));
        assert_eq!(1u64.hours(), TimeDelta::hours(1));
        assert_eq!(500u64.millis(), TimeDelta::milliseconds(500));
        assert_eq!(200u64.micros(), TimeDelta::microseconds(200));
        assert_eq!(100u64.nanos(), TimeDelta::nanoseconds(100));
        assert_eq!(7u64.days(), TimeDelta::days(7));
    }

    #[test]
    fn i64_conversions() {
        assert_eq!((-5i64).seconds(), TimeDelta::seconds(-5));
        assert_eq!(10i64.minutes(), TimeDelta::minutes(10));
        assert_eq!(24i64.hours(), TimeDelta::hours(24));
        assert_eq!(30i64.days(), TimeDelta::days(30));
    }

    #[test]
    fn u32_conversions() {
        assert_eq!(60u32.seconds(), TimeDelta::seconds(60));
        assert_eq!(1u32.days(), TimeDelta::days(1));
    }

    #[test]
    fn i32_conversions() {
        assert_eq!((-1i32).days(), TimeDelta::days(-1));
        assert_eq!(5i32.hours(), TimeDelta::hours(5));
    }
}
