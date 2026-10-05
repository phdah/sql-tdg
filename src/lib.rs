//! Core Rust library for sql-tdg.

#![forbid(unsafe_code)]

pub mod generator;
pub mod protocol;
mod protocol_value;
pub mod solver;
pub mod table;
pub mod test_case;
pub mod types;

pub use generator::{Generator, GeneratorError};
pub use protocol::{
    GeneratedData, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError,
    generate_classified_from_bundle, generate_classified_from_bundle_at_boundary,
    generate_classified_from_sql, generate_classified_from_sql_at_boundary, generate_from_bundle,
    generate_from_bundle_at_boundary, generate_from_sql, generate_from_sql_at_boundary,
};
pub use solver::{
    BoolDomain, IntDomain, SolverError, TimestampDomain, from_unix, parse_time, to_date,
    to_timestamp,
};
pub use sql_semantic_protocol::{DataType, RelationSchema, SchemaColumn};
pub use table::{Dim, Table, TableError, TableValue, TimestampColumns};
pub use test_case::{
    ApprovedResult, BoundaryKind, ClassifiedRowCounts, GeneratedRelation, GenerationBoundary,
    ProtocolSnapshot, QueryResult, ResultColumn, ResultOrdering, TargetKind, TestCase,
    TestCaseError, TestCaseMetadata, TestTarget, VerificationError, WorkloadIdentity, WorkloadKind,
};
pub use types::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    IntervalError, TimestampConstraint,
};
