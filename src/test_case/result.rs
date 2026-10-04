use std::error::Error;
use std::fmt;

use super::{TestCaseError, TestCaseMetadata, required_string};

/// One result-set column as reported by the execution backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultColumn {
    name: String,
    data_type: String,
}

impl ResultColumn {
    /// Creates a named result column with a canonical backend type description.
    pub fn new(
        name: impl Into<String>,
        data_type: impl Into<String>,
    ) -> Result<Self, TestCaseError> {
        Ok(Self {
            name: required_string(name, "result column name")?,
            data_type: required_string(data_type, "result column type")?,
        })
    }

    /// Returns the column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the canonical backend type description.
    pub fn data_type(&self) -> &str {
        &self.data_type
    }
}

/// Complete observed query result using canonical lossless cell encodings supplied by the backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryResult {
    columns: Vec<ResultColumn>,
    rows: Vec<Vec<String>>,
}

impl QueryResult {
    /// Creates a result after validating every row against the declared column count.
    pub fn new(
        columns: Vec<ResultColumn>,
        rows: Vec<Vec<String>>,
    ) -> Result<Self, TestCaseError> {
        let expected = columns.len();
        for (row_index, row) in rows.iter().enumerate() {
            if row.len() != expected {
                return Err(TestCaseError::InvalidResultRowWidth {
                    row_index,
                    expected,
                    actual: row.len(),
                });
            }
        }
        Ok(Self { columns, rows })
    }

    /// Returns result columns in execution order.
    pub fn columns(&self) -> &[ResultColumn] {
        &self.columns
    }

    /// Returns result rows in execution order.
    pub fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }
}

/// Ordering semantics used when comparing a current result with an approved result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultOrdering {
    /// Result row order is observable and must match exactly.
    Ordered,
    /// Result row order is not observable; rows compare as a deterministic multiset.
    Unordered,
}

/// Explicitly approved expected result, separate from the current workload text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovedResult {
    ordering: ResultOrdering,
    result: QueryResult,
}

impl ApprovedResult {
    /// Creates an approved result snapshot with explicit ordering semantics.
    pub const fn new(result: QueryResult, ordering: ResultOrdering) -> Self {
        Self { ordering, result }
    }

    /// Returns the comparison ordering contract.
    pub const fn ordering(&self) -> ResultOrdering {
        self.ordering
    }

    /// Returns the approved complete result.
    pub const fn result(&self) -> &QueryResult {
        &self.result
    }

    /// Compares a current result without mutating or re-approving the snapshot.
    pub fn matches(&self, current: &QueryResult) -> bool {
        if self.result.columns != current.columns {
            return false;
        }

        match self.ordering {
            ResultOrdering::Ordered => self.result.rows == current.rows,
            ResultOrdering::Unordered => {
                let mut expected = self.result.rows.clone();
                let mut actual = current.rows.clone();
                expected.sort();
                actual.sort();
                expected == actual
            }
        }
    }
}

/// Durable test case containing reproducibility metadata and an optional explicit approval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCase {
    metadata: TestCaseMetadata,
    approved_result: Option<ApprovedResult>,
}

impl TestCase {
    /// Creates an unapproved test case. Verification cannot approve results implicitly.
    pub const fn new(metadata: TestCaseMetadata) -> Self {
        Self {
            metadata,
            approved_result: None,
        }
    }

    /// Returns reproducibility metadata.
    pub const fn metadata(&self) -> &TestCaseMetadata {
        &self.metadata
    }

    /// Returns the currently approved result, if one has been explicitly recorded.
    pub const fn approved_result(&self) -> Option<&ApprovedResult> {
        self.approved_result.as_ref()
    }

    /// Explicitly approves or re-approves a complete result snapshot.
    pub fn approve_result(&mut self, result: QueryResult, ordering: ResultOrdering) {
        self.approved_result = Some(ApprovedResult::new(result, ordering));
    }

    /// Verifies a current result against the existing approval without mutating the test case.
    pub fn verify_result(&self, current: &QueryResult) -> Result<(), VerificationError> {
        let approved = self
            .approved_result
            .as_ref()
            .ok_or(VerificationError::MissingApproval)?;
        if approved.matches(current) {
            Ok(())
        } else {
            Err(VerificationError::ResultMismatch)
        }
    }
}

/// Verification failures that never mutate the approved expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationError {
    /// No expected result has been explicitly approved.
    MissingApproval,
    /// The complete current result differs from the approved result under its ordering contract.
    ResultMismatch,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingApproval => formatter.write_str("test case has no approved result"),
            Self::ResultMismatch => formatter.write_str("current result differs from approval"),
        }
    }
}

impl Error for VerificationError {}
