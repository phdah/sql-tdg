//! Durable test-case and expected-result contracts.
//!
//! This module records the inputs required to reproduce generated data without owning SQL
//! semantics. The normalized SQL Semantic Protocol document is stored opaquely and interpreted
//! only by the protocol boundary when generation runs.

use std::error::Error;
use std::fmt;

mod codec;
mod metadata;
mod model;
mod result;

pub use metadata::TestCaseMetadata;
pub use model::{
    BoundaryKind, ClassifiedRowCounts, GeneratedRelation, GenerationBoundary, ProtocolSnapshot,
    TargetKind, TestTarget, WorkloadIdentity, WorkloadKind,
};
pub use result::{
    ApprovedResult, QueryResult, ResultColumn, ResultOrdering, TestCase, VerificationError,
};

/// Errors returned while constructing or decoding durable test-case data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestCaseError {
    /// A required string field was empty or whitespace-only.
    EmptyField {
        /// Field that requires a value.
        field: &'static str,
    },
    /// A required collection contained no entries.
    EmptyCollection {
        /// Collection that requires at least one entry.
        field: &'static str,
    },
    /// A relation identifier was repeated where identities must be unique.
    DuplicateRelation {
        /// Duplicated relation identity.
        relation: String,
    },
    /// An intermediate boundary refers to a relation not present in generated relations.
    MissingBoundaryRelation {
        /// Missing relation identity.
        relation: String,
    },
    /// A result row does not contain exactly one value per declared column.
    InvalidResultRowWidth {
        /// Zero-based row index.
        row_index: usize,
        /// Number of declared result columns.
        expected: usize,
        /// Number of values found in the row.
        actual: usize,
    },
    /// Serialized metadata does not match the stable v1 contract.
    InvalidMetadata {
        /// Explanation of the invalid record.
        message: String,
    },
}

impl fmt::Display for TestCaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField { field } => write!(formatter, "{field} must not be empty"),
            Self::EmptyCollection { field } => write!(formatter, "{field} must not be empty"),
            Self::DuplicateRelation { relation } => {
                write!(formatter, "relation {relation:?} is declared more than once")
            }
            Self::MissingBoundaryRelation { relation } => write!(
                formatter,
                "intermediate boundary relation {relation:?} is not generated"
            ),
            Self::InvalidResultRowWidth {
                row_index,
                expected,
                actual,
            } => write!(
                formatter,
                "result row {row_index} has {actual} values, expected {expected}"
            ),
            Self::InvalidMetadata { message } => {
                write!(formatter, "invalid test-case metadata: {message}")
            }
        }
    }
}

impl Error for TestCaseError {}

pub(super) fn required_string(
    value: impl Into<String>,
    field: &'static str,
) -> Result<String, TestCaseError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(TestCaseError::EmptyField { field });
    }
    Ok(value)
}

pub(super) fn reject_duplicates(values: &[String]) -> Result<(), TestCaseError> {
    let mut previous: Option<&str> = None;
    for value in values {
        if previous == Some(value.as_str()) {
            return Err(TestCaseError::DuplicateRelation {
                relation: value.clone(),
            });
        }
        previous = Some(value);
    }
    Ok(())
}
