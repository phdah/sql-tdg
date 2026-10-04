//! Timestamp parsing and domain narrowing.

use chrono::{DateTime, NaiveDate, Utc};

use crate::{IntConstraint, TimestampConstraint};

use super::{IntDomain, SolverError};

const MIN_TIMESTAMP: i32 = 0;
const MAX_TIMESTAMP: i32 = i32::MAX;

/// Allowed timestamp values represented as Unix seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimestampDomain {
    seconds: IntDomain,
}

impl TimestampDomain {
    /// Creates the supported timestamp domain.
    pub fn new() -> Self {
        Self {
            seconds: IntDomain::with_bounds(MIN_TIMESTAMP, MAX_TIMESTAMP)
                .expect("timestamp bounds form a valid non-empty domain"),
        }
    }

    /// Creates a timestamp domain from inclusive Unix-second intervals.
    pub(crate) fn from_intervals(intervals: Vec<crate::Interval>) -> Result<Self, SolverError> {
        let seconds = IntDomain::from_intervals(intervals)?;
        if seconds.total_min() < MIN_TIMESTAMP {
            return Err(SolverError::UnsatisfiableDomain);
        }
        Ok(Self { seconds })
    }

    /// Applies one timestamp constraint to the domain.
    pub fn apply(&mut self, constraint: TimestampConstraint) -> Result<(), SolverError> {
        self.seconds.apply(match constraint {
            TimestampConstraint::Equal(value) => IntConstraint::Equal(value),
            TimestampConstraint::NotEqual(value) => IntConstraint::NotEqual(value),
            TimestampConstraint::LessThan(value) => IntConstraint::LessThan(value),
            TimestampConstraint::LessThanOrEqual(value) => IntConstraint::LessThanOrEqual(value),
            TimestampConstraint::GreaterThan(value) => IntConstraint::GreaterThan(value),
            TimestampConstraint::GreaterThanOrEqual(value) => {
                IntConstraint::GreaterThanOrEqual(value)
            }
        })
    }

    /// Returns the allowed Unix-second intervals in ascending order.
    pub fn intervals(&self) -> &[crate::Interval] {
        self.seconds.intervals()
    }

    /// Returns the smallest currently allowed Unix second.
    pub const fn total_min(&self) -> i32 {
        self.seconds.total_min()
    }

    /// Returns the largest currently allowed Unix second.
    pub const fn total_max(&self) -> i32 {
        self.seconds.total_max()
    }

    /// Returns the number of currently allowed Unix-second values.
    pub fn value_count(&self) -> usize {
        self.seconds.value_count()
    }

    /// Returns the timestamp at the zero-based position across the domain.
    pub fn value_at(&self, index: usize) -> Result<DateTime<Utc>, SolverError> {
        let seconds = self.seconds.value_at(index)?;
        from_unix(seconds)
    }
}

impl Default for TimestampDomain {
    fn default() -> Self {
        Self::new()
    }
}

/// Parses an RFC3339 timestamp literal into Unix seconds.
pub fn to_timestamp(timestamp: &str) -> Result<i32, SolverError> {
    let value = trim_quotes(timestamp);
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|_| SolverError::InvalidTimestamp {
            value: value.to_owned(),
        })?;
    checked_seconds(parsed.timestamp(), value)
}

/// Parses a `YYYY-MM-DD` date literal at midnight UTC into Unix seconds.
pub fn to_date(date: &str) -> Result<i32, SolverError> {
    let value = trim_quotes(date);
    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
        SolverError::InvalidTimestamp {
            value: value.to_owned(),
        }
    })?;
    let midnight = parsed
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| SolverError::InvalidTimestamp {
            value: value.to_owned(),
        })?;
    checked_seconds(midnight.and_utc().timestamp(), value)
}

/// Parses either an RFC3339 timestamp or a `YYYY-MM-DD` date into Unix seconds.
pub fn parse_time(timestamp: &str) -> Result<i32, SolverError> {
    let value = trim_quotes(timestamp);

    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return checked_seconds(parsed.timestamp(), value);
    }

    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        let midnight =
            parsed
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| SolverError::InvalidTimestamp {
                    value: value.to_owned(),
                })?;
        return checked_seconds(midnight.and_utc().timestamp(), value);
    }

    Err(SolverError::InvalidTimestamp {
        value: value.to_owned(),
    })
}

/// Converts Unix seconds to a UTC timestamp.
pub fn from_unix(timestamp: i32) -> Result<DateTime<Utc>, SolverError> {
    DateTime::from_timestamp(i64::from(timestamp), 0).ok_or_else(|| {
        SolverError::TimestampOutOfRange {
            value: timestamp.to_string(),
        }
    })
}

fn checked_seconds(seconds: i64, value: &str) -> Result<i32, SolverError> {
    i32::try_from(seconds).map_err(|_| SolverError::TimestampOutOfRange {
        value: value.to_owned(),
    })
}

fn trim_quotes(value: &str) -> &str {
    value.trim_matches(|character| character == '\'' || character == '"')
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use crate::TimestampConstraint;

    use super::{SolverError, TimestampDomain, from_unix, parse_time, to_date, to_timestamp};

    #[test]
    fn single_apply_matches_supported_operators() {
        let cases = [
            (TimestampConstraint::Equal(3), vec![(3, 3)], 3, 3),
            (
                TimestampConstraint::NotEqual(3),
                vec![(0, 2), (4, i32::MAX)],
                0,
                i32::MAX,
            ),
            (TimestampConstraint::LessThan(3), vec![(0, 2)], 0, 2),
            (TimestampConstraint::LessThanOrEqual(3), vec![(0, 3)], 0, 3),
            (
                TimestampConstraint::GreaterThan(3),
                vec![(4, i32::MAX)],
                4,
                i32::MAX,
            ),
            (
                TimestampConstraint::GreaterThanOrEqual(3),
                vec![(3, i32::MAX)],
                3,
                i32::MAX,
            ),
        ];

        for (constraint, expected_intervals, expected_min, expected_max) in cases {
            let mut domain = TimestampDomain::new();
            domain.apply(constraint).expect("constraint should apply");

            let actual_intervals: Vec<_> = domain
                .intervals()
                .iter()
                .map(|interval| (interval.min(), interval.max()))
                .collect();
            assert_eq!(actual_intervals, expected_intervals);
            assert_eq!(domain.total_min(), expected_min);
            assert_eq!(domain.total_max(), expected_max);
        }
    }

    #[test]
    fn multi_apply_keeps_only_values_matching_every_constraint() {
        let mut domain = TimestampDomain::new();
        for constraint in [
            TimestampConstraint::Equal(3),
            TimestampConstraint::GreaterThan(-10),
            TimestampConstraint::GreaterThanOrEqual(0),
            TimestampConstraint::LessThan(200),
            TimestampConstraint::LessThanOrEqual(150),
            TimestampConstraint::NotEqual(100),
        ] {
            domain
                .apply(constraint)
                .expect("constraints should overlap");
        }

        assert_eq!(domain.intervals().len(), 1);
        assert_eq!(domain.intervals()[0].min(), 3);
        assert_eq!(domain.intervals()[0].max(), 3);
        assert_eq!(domain.total_min(), 3);
        assert_eq!(domain.total_max(), 3);
    }

    #[test]
    fn contradictory_constraints_are_rejected() {
        let mut domain = TimestampDomain::new();
        domain
            .apply(TimestampConstraint::NotEqual(5))
            .expect("excluding one value should succeed");

        let error = domain
            .apply(TimestampConstraint::Equal(5))
            .expect_err("excluded timestamp must not be restored");

        assert_eq!(error, SolverError::UnsatisfiableDomain);
    }

    #[test]
    fn to_date_matches_supported_date_format() {
        assert_eq!(to_date("2013-06-17"), Ok(1_371_427_200));
        assert_eq!(to_date("'2013-06-17'"), Ok(1_371_427_200));
    }

    #[test]
    fn to_timestamp_matches_supported_rfc3339_format() {
        assert_eq!(to_timestamp("2013-06-17T12:25:04Z"), Ok(1_371_471_904));
        assert_eq!(to_timestamp("\"2013-06-17T12:25:04Z\""), Ok(1_371_471_904));
    }

    #[test]
    fn parse_time_accepts_date_and_timestamp_and_rejects_other_formats() {
        assert_eq!(parse_time("2013-06-17"), Ok(1_371_427_200));
        assert_eq!(parse_time("2013-06-17T12:25:04Z"), Ok(1_371_471_904));
        assert_eq!(
            parse_time("17/06/2013"),
            Err(SolverError::InvalidTimestamp {
                value: "17/06/2013".to_owned(),
            })
        );
    }

    #[test]
    fn from_unix_matches_supported_date_and_timestamp_values() {
        let cases = [
            (
                1_371_427_200,
                Utc.with_ymd_and_hms(2013, 6, 17, 0, 0, 0)
                    .single()
                    .expect("valid UTC date"),
            ),
            (
                1_371_471_904,
                Utc.with_ymd_and_hms(2013, 6, 17, 12, 25, 4)
                    .single()
                    .expect("valid UTC timestamp"),
            ),
            (
                to_date("2013-06-17").expect("supported date"),
                Utc.with_ymd_and_hms(2013, 6, 17, 0, 0, 0)
                    .single()
                    .expect("valid UTC date"),
            ),
            (
                to_timestamp("2013-06-17T12:25:04Z").expect("supported timestamp"),
                Utc.with_ymd_and_hms(2013, 6, 17, 12, 25, 4)
                    .single()
                    .expect("valid UTC timestamp"),
            ),
        ];

        for (seconds, expected) in cases {
            assert_eq!(from_unix(seconds), Ok(expected));
        }
    }

    #[test]
    fn value_at_converts_unix_seconds_to_utc_datetime() {
        let mut domain = TimestampDomain::new();
        domain
            .apply(TimestampConstraint::Equal(1_371_427_200))
            .expect("timestamp should be in supported range");

        let actual = domain
            .value_at(0)
            .expect("single value should be available");
        let expected = Utc
            .with_ymd_and_hms(2013, 6, 17, 0, 0, 0)
            .single()
            .expect("valid UTC date");

        assert_eq!(actual, expected);
    }
}
