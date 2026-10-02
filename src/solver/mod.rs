//! Constraint solving for supported column domains.

mod bools;
mod integers;
mod timestamp;

pub use bools::BoolDomain;
pub use integers::IntDomain;
pub use timestamp::{TimestampDomain, from_unix, parse_time, to_date, to_timestamp};

use std::error::Error;
use std::fmt;

/// Errors returned while narrowing or reading a solver domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolverError {
    /// No value satisfies all applied constraints.
    UnsatisfiableDomain,
    /// A requested position is outside the values allowed by the domain.
    ValueIndexOutOfRange {
        /// Requested zero-based position.
        index: usize,
        /// Number of values currently allowed by the domain.
        value_count: usize,
    },
    /// A date or timestamp literal is not supported.
    InvalidTimestamp {
        /// Original literal after surrounding quotes were removed.
        value: String,
    },
    /// A parsed timestamp cannot be represented by the supported Unix-second range.
    TimestampOutOfRange {
        /// Original literal after surrounding quotes were removed.
        value: String,
    },
}

impl fmt::Display for SolverError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsatisfiableDomain => formatter.write_str("domain has no allowed values"),
            Self::ValueIndexOutOfRange { index, value_count } => write!(
                formatter,
                "value index {index} is outside domain with {value_count} values"
            ),
            Self::InvalidTimestamp { value } => {
                write!(formatter, "could not parse timestamp or date: {value}")
            }
            Self::TimestampOutOfRange { value } => {
                write!(formatter, "timestamp is outside supported range: {value}")
            }
        }
    }
}

impl Error for SolverError {}
