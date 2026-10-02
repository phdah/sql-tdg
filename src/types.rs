//! Shared domain types used across the Rust implementation.
//!
//! This module owns project-level types only. It intentionally has no dependency on parser,
//! solver, interop, table, or generator modules.

use std::error::Error;
use std::fmt;

/// Supported column types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColumnType {
    /// Signed 32-bit integer values.
    Int,
    /// Timestamp values represented as Unix seconds while solving.
    Timestamp,
    /// Boolean values.
    Bool,
    /// UTF-8 string values.
    String,
}

impl ColumnType {
    /// Returns the stable textual representation used by the Go implementation.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::Timestamp => "timestamp",
            Self::Bool => "bool",
            Self::String => "string",
        }
    }
}

impl fmt::Display for ColumnType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Inclusive integer interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    min: i32,
    max: i32,
}

impl Interval {
    /// Creates an inclusive interval when the lower bound does not exceed the upper bound.
    pub fn new(min: i32, max: i32) -> Result<Self, IntervalError> {
        if min > max {
            return Err(IntervalError { min, max });
        }

        Ok(Self { min, max })
    }

    /// Returns the inclusive lower bound.
    pub const fn min(self) -> i32 {
        self.min
    }

    /// Returns the inclusive upper bound.
    pub const fn max(self) -> i32 {
        self.max
    }

    /// Reports whether the value is inside the inclusive interval.
    pub const fn contains(self, value: i32) -> bool {
        self.min <= value && value <= self.max
    }
}

/// Error returned when interval bounds are invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntervalError {
    min: i32,
    max: i32,
}

impl IntervalError {
    /// Returns the attempted lower bound.
    pub const fn min(self) -> i32 {
        self.min
    }

    /// Returns the attempted upper bound.
    pub const fn max(self) -> i32 {
        self.max
    }
}

impl fmt::Display for IntervalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "interval min {} is larger than max {}",
            self.min, self.max
        )
    }
}

impl Error for IntervalError {}

/// Constraints supported for integer columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntConstraint {
    /// Value must equal the operand.
    Equal(i32),
    /// Value must not equal the operand.
    NotEqual(i32),
    /// Value must be less than the operand.
    LessThan(i32),
    /// Value must be less than or equal to the operand.
    LessThanOrEqual(i32),
    /// Value must be greater than the operand.
    GreaterThan(i32),
    /// Value must be greater than or equal to the operand.
    GreaterThanOrEqual(i32),
}

/// Constraints supported for timestamp columns.
///
/// Operands use Unix seconds, matching the current Go solver representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampConstraint {
    /// Value must equal the operand.
    Equal(i32),
    /// Value must not equal the operand.
    NotEqual(i32),
    /// Value must be less than the operand.
    LessThan(i32),
    /// Value must be less than or equal to the operand.
    LessThanOrEqual(i32),
    /// Value must be greater than the operand.
    GreaterThan(i32),
    /// Value must be greater than or equal to the operand.
    GreaterThanOrEqual(i32),
}

/// Constraints supported for boolean columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolConstraint {
    /// Value must be true.
    IsTrue,
    /// Value must be false.
    IsFalse,
}

/// Type-safe shared constraint representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constraint {
    /// Integer constraint.
    Int(IntConstraint),
    /// Timestamp constraint using Unix seconds.
    Timestamp(TimestampConstraint),
    /// Boolean constraint.
    Bool(BoolConstraint),
}

impl Constraint {
    /// Returns the column type this constraint can be applied to.
    pub const fn column_type(self) -> ColumnType {
        match self {
            Self::Int(_) => ColumnType::Int,
            Self::Timestamp(_) => ColumnType::Timestamp,
            Self::Bool(_) => ColumnType::Bool,
        }
    }
}

/// Column definition and its validated constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    name: String,
    column_type: ColumnType,
    constraints: Vec<Constraint>,
}

impl Column {
    /// Creates a column without constraints.
    pub fn new(name: impl Into<String>, column_type: ColumnType) -> Self {
        Self {
            name: name.into(),
            column_type,
            constraints: Vec::new(),
        }
    }

    /// Returns the column name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the declared column type.
    pub const fn column_type(&self) -> ColumnType {
        self.column_type
    }

    /// Returns the constraints in insertion order.
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }

    /// Adds a constraint when it matches the declared column type.
    pub fn add_constraint(&mut self, constraint: Constraint) -> Result<(), ColumnError> {
        let constraint_type = constraint.column_type();
        if constraint_type != self.column_type {
            return Err(ColumnError::ConstraintTypeMismatch {
                column_type: self.column_type,
                constraint_type,
            });
        }

        self.constraints.push(constraint);
        Ok(())
    }
}

/// Error returned when constructing or updating a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnError {
    /// The constraint belongs to a different column type.
    ConstraintTypeMismatch {
        /// Type declared by the column.
        column_type: ColumnType,
        /// Type required by the constraint.
        constraint_type: ColumnType,
    },
}

impl fmt::Display for ColumnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConstraintTypeMismatch {
                column_type,
                constraint_type,
            } => write!(
                formatter,
                "cannot apply {constraint_type} constraint to {column_type} column"
            ),
        }
    }
}

impl Error for ColumnError {}
