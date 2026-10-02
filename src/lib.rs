//! Core Rust library for sql-tdg.

#![forbid(unsafe_code)]

pub mod parser;
pub mod solver;
pub mod types;

pub use parser::{
    ConditionIR, ConditionOperator, JoinIR, JoinKind, ParserError, QueryIR, parse_query,
};
pub use solver::{
    BoolDomain, IntDomain, SolverError, TimestampDomain, from_unix, parse_time, to_date,
    to_timestamp,
};
pub use types::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    IntervalError, TimestampConstraint,
};
