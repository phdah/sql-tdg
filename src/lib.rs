//! Core Rust library for sql-tdg.

#![forbid(unsafe_code)]

pub mod types;

pub use types::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    IntervalError, TimestampConstraint,
};
