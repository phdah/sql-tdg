//! Mapping from parsed condition IR to typed schema constraints.

use std::error::Error;
use std::fmt;

use crate::parser::{ConditionIR, ConditionOperator, QueryIR};
use crate::solver::{SolverError, parse_time};
use crate::table::Table;
use crate::types::{
    BoolConstraint, ColumnError, ColumnType, Constraint, IntConstraint, TimestampConstraint,
};

#[cfg(test)]
mod tests;

/// Errors returned while applying parsed conditions to a table schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteropError {
    /// A condition references a column that is not present in the table schema.
    UnknownColumn {
        /// Referenced column name.
        column: String,
    },
    /// An integer condition operand cannot be represented as an i32.
    InvalidInteger {
        /// Column receiving the condition.
        column: String,
        /// Parsed operand text.
        value: String,
    },
    /// A boolean condition operand is not a supported boolean literal.
    InvalidBoolean {
        /// Column receiving the condition.
        column: String,
        /// Parsed operand text.
        value: String,
    },
    /// A timestamp condition operand cannot be parsed into the supported timestamp domain.
    InvalidTimestamp {
        /// Column receiving the condition.
        column: String,
        /// Parsed operand text.
        value: String,
        /// Timestamp parsing error.
        source: SolverError,
    },
    /// The column type has no constraint mapping.
    UnsupportedColumnType {
        /// Column receiving the condition.
        column: String,
        /// Unsupported schema type.
        column_type: ColumnType,
    },
    /// The operator has no constraint mapping for the column type.
    UnsupportedOperator {
        /// Column receiving the condition.
        column: String,
        /// Schema type of the column.
        column_type: ColumnType,
        /// Unsupported condition operator.
        operator: ConditionOperator,
    },
    /// A mapped constraint does not match the schema column type.
    ConstraintTypeMismatch {
        /// Column receiving the condition.
        column: String,
        /// Column validation error.
        source: ColumnError,
    },
}

impl fmt::Display for InteropError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownColumn { column } => write!(formatter, "unknown column {column:?}"),
            Self::InvalidInteger { column, value } => {
                write!(formatter, "invalid integer {value:?} for column {column:?}")
            }
            Self::InvalidBoolean { column, value } => {
                write!(formatter, "invalid boolean {value:?} for column {column:?}")
            }
            Self::InvalidTimestamp { column, value, .. } => {
                write!(formatter, "invalid timestamp {value:?} for column {column:?}")
            }
            Self::UnsupportedColumnType {
                column,
                column_type,
            } => write!(
                formatter,
                "unsupported column type {column_type} for column {column:?}"
            ),
            Self::UnsupportedOperator {
                column,
                column_type,
                operator,
            } => write!(
                formatter,
                "unsupported operator {operator} for {column_type} column {column:?}"
            ),
            Self::ConstraintTypeMismatch { column, .. } => {
                write!(formatter, "constraint type mismatch for column {column:?}")
            }
        }
    }
}

impl Error for InteropError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidTimestamp { source, .. } => Some(source),
            Self::ConstraintTypeMismatch { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Applies every query condition to its matching table schema column.
///
/// Constraint mapping is completed before the schema is mutated, so an invalid condition leaves
/// the table unchanged.
pub fn apply_conditions(query: &QueryIR, table: &mut Table) -> Result<(), InteropError> {
    let mut mapped = Vec::with_capacity(query.conditions().len());

    for condition in query.conditions() {
        let column = table
            .schema()
            .iter()
            .find(|column| column.name() == condition.left())
            .ok_or_else(|| InteropError::UnknownColumn {
                column: condition.left().to_owned(),
            })?;

        let constraint = make_constraint(column.name(), column.column_type(), condition)?;
        mapped.push((column.name().to_owned(), constraint));
    }

    for (column_name, constraint) in mapped {
        let column = table
            .schema_column_mut(&column_name)
            .ok_or_else(|| InteropError::UnknownColumn {
                column: column_name.clone(),
            })?;
        column
            .add_constraint(constraint)
            .map_err(|source| InteropError::ConstraintTypeMismatch {
                column: column_name,
                source,
            })?;
    }

    Ok(())
}

fn make_constraint(
    column: &str,
    column_type: ColumnType,
    condition: &ConditionIR,
) -> Result<Constraint, InteropError> {
    match column_type {
        ColumnType::Int => {
            let value = condition
                .right()
                .parse::<i32>()
                .map_err(|_| InteropError::InvalidInteger {
                    column: column.to_owned(),
                    value: condition.right().to_owned(),
                })?;
            let constraint = match condition.operator() {
                ConditionOperator::Equal => IntConstraint::Equal(value),
                ConditionOperator::NotEqual => IntConstraint::NotEqual(value),
                ConditionOperator::LessThan => IntConstraint::LessThan(value),
                ConditionOperator::LessThanOrEqual => IntConstraint::LessThanOrEqual(value),
                ConditionOperator::GreaterThan => IntConstraint::GreaterThan(value),
                ConditionOperator::GreaterThanOrEqual => IntConstraint::GreaterThanOrEqual(value),
                ConditionOperator::Boolean => {
                    return Err(InteropError::UnsupportedOperator {
                        column: column.to_owned(),
                        column_type,
                        operator: condition.operator(),
                    });
                }
            };
            Ok(Constraint::Int(constraint))
        }
        ColumnType::Bool => {
            let constraint = match condition.operator() {
                ConditionOperator::Boolean => BoolConstraint::IsTrue,
                ConditionOperator::Equal => match condition.right() {
                    "true" => BoolConstraint::IsTrue,
                    "false" => BoolConstraint::IsFalse,
                    value => {
                        return Err(InteropError::InvalidBoolean {
                            column: column.to_owned(),
                            value: value.to_owned(),
                        });
                    }
                },
                operator => {
                    return Err(InteropError::UnsupportedOperator {
                        column: column.to_owned(),
                        column_type,
                        operator,
                    });
                }
            };
            Ok(Constraint::Bool(constraint))
        }
        ColumnType::Timestamp => {
            if condition.operator() == ConditionOperator::Boolean {
                return Err(InteropError::UnsupportedOperator {
                    column: column.to_owned(),
                    column_type,
                    operator: condition.operator(),
                });
            }

            let value =
                parse_time(condition.right()).map_err(|source| InteropError::InvalidTimestamp {
                    column: column.to_owned(),
                    value: condition.right().to_owned(),
                    source,
                })?;
            let constraint = match condition.operator() {
                ConditionOperator::Equal => TimestampConstraint::Equal(value),
                ConditionOperator::NotEqual => TimestampConstraint::NotEqual(value),
                ConditionOperator::LessThan => TimestampConstraint::LessThan(value),
                ConditionOperator::LessThanOrEqual => TimestampConstraint::LessThanOrEqual(value),
                ConditionOperator::GreaterThan => TimestampConstraint::GreaterThan(value),
                ConditionOperator::GreaterThanOrEqual => {
                    TimestampConstraint::GreaterThanOrEqual(value)
                }
                ConditionOperator::Boolean => unreachable!("boolean operator returned above"),
            };
            Ok(Constraint::Timestamp(constraint))
        }
        ColumnType::String => Err(InteropError::UnsupportedColumnType {
            column: column.to_owned(),
            column_type,
        }),
    }
}
