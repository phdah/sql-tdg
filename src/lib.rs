//! Core Rust library for sql-tdg.

#![forbid(unsafe_code)]

mod case_coverage;
mod constraint_generation;
pub use case_coverage::{CaseCoverageFinding, CaseCoverageStatus};
pub mod export;
pub mod generator;
pub mod protocol;
mod protocol_value;
pub mod solver;
pub mod table;
pub mod test_case;
pub mod types;

pub use export::{ExportError, record_batch, write_csv, write_parquet};
pub use generator::{Generator, GeneratorError};
pub use protocol::{
    GeneratedData, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError,
    SqlGenerationSettings, generate_classified_from_bundle,
    generate_classified_from_bundle_at_boundary, generate_classified_from_sql,
    generate_classified_from_sql_at_boundary,
    generate_classified_from_sql_at_boundary_with_assumptions,
    generate_classified_from_sql_with_assumptions, generate_from_bundle,
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
