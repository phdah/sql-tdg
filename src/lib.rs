//! Core Rust library for sql-tdg.

#![forbid(unsafe_code)]

pub mod generator;
pub mod protocol;
pub mod solver;
pub mod table;
pub mod types;

pub use generator::{Generator, GeneratorError};
pub use protocol::{
    GeneratedData, OutcomeSelector, ProtocolGenerationError, generate_from_bundle,
    generate_from_sql,
};
pub use sql_semantic_protocol::{RelationSchema, ScalarType, SchemaColumn};
pub use solver::{
    BoolDomain, IntDomain, SolverError, TimestampDomain, from_unix, parse_time, to_date,
    to_timestamp,
};
pub use table::{Dim, Table, TableError, TableValue, TimestampColumns};
pub use types::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    IntervalError, TimestampConstraint,
};
