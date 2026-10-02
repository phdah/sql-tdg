//! Rust implementation of sql-tdg.
//!
//! The Go implementation remains the behavioral reference until the Rust migration reaches
//! end-to-end parity and the final cutover is completed.

#![forbid(unsafe_code)]

pub mod types;

pub use types::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    IntervalError, TimestampConstraint,
};
