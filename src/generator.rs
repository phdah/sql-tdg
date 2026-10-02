//! Deterministic seeded value generation into table storage.

use std::error::Error;
use std::fmt;

use rand_chacha::ChaCha8Rng;
use rand_core::{Rng, SeedableRng};

use crate::solver::{BoolDomain, IntDomain, SolverError, TimestampDomain};
use crate::table::{Table, TableError, TableValue};
use crate::types::{Column, ColumnType, Constraint};

/// Errors returned while preparing or generating table values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorError {
    /// The schema contains a column type that has no generation domain.
    UnsupportedColumnType {
        /// Column that cannot be generated.
        column: String,
        /// Unsupported schema type.
        column_type: ColumnType,
    },
    /// A column contains a constraint for another column type.
    ConstraintTypeMismatch {
        /// Column containing the invalid constraint.
        column: String,
        /// Type declared by the column.
        column_type: ColumnType,
        /// Type required by the constraint.
        constraint_type: ColumnType,
    },
    /// Applying constraints or reading a generated value failed.
    Solver {
        /// Column whose domain failed.
        column: String,
        /// Solver error for the domain.
        source: SolverError,
    },
    /// A prepared domain unexpectedly contains no values.
    EmptyDomain {
        /// Column whose domain is empty.
        column: String,
    },
    /// A prepared domain cannot be indexed by the seeded random source.
    DomainTooLarge {
        /// Column whose domain is too large.
        column: String,
        /// Number of allowed values in the domain.
        value_count: usize,
    },
    /// Appending a generated value to table storage failed.
    Table {
        /// Column receiving the generated value.
        column: String,
        /// Table operation error.
        source: TableError,
    },
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedColumnType {
                column,
                column_type,
            } => write!(
                formatter,
                "unsupported column type {column_type} for column {column:?}"
            ),
            Self::ConstraintTypeMismatch {
                column,
                column_type,
                constraint_type,
            } => write!(
                formatter,
                "{column_type} column {column:?} cannot use {constraint_type} constraint"
            ),
            Self::Solver { column, .. } => {
                write!(formatter, "could not prepare or sample column {column:?}")
            }
            Self::EmptyDomain { column } => {
                write!(formatter, "column {column:?} has no values to generate")
            }
            Self::DomainTooLarge {
                column,
                value_count,
            } => write!(
                formatter,
                "column {column:?} domain with {value_count} values is too large to sample"
            ),
            Self::Table { column, .. } => {
                write!(
                    formatter,
                    "could not append generated value to column {column:?}"
                )
            }
        }
    }
}

impl Error for GeneratorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Solver { source, .. } => Some(source),
            Self::Table { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Deterministic generator for supported table column types.
#[derive(Debug, Clone, Copy, Default)]
pub struct Generator;

impl Generator {
    /// Creates a generator.
    pub const fn new() -> Self {
        Self
    }

    /// Appends the declared number of rows using a deterministic seeded random stream.
    ///
    /// Columns are sampled in schema declaration order for each row. All column domains are
    /// prepared before any value is appended, so invalid constraints do not leave partial output.
    pub fn generate(&self, table: &mut Table, seed: u64) -> Result<(), GeneratorError> {
        let plans = table
            .schema()
            .iter()
            .map(ColumnPlan::from_column)
            .collect::<Result<Vec<_>, _>>()?;
        let rows = table.dim().rows();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);

        for _ in 0..rows {
            for plan in &plans {
                let value = plan.sample(&mut rng)?;
                table
                    .append(&plan.name, value)
                    .map_err(|source| GeneratorError::Table {
                        column: plan.name.clone(),
                        source,
                    })?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ColumnPlan {
    name: String,
    domain: GenerationDomain,
}

impl ColumnPlan {
    fn from_column(column: &Column) -> Result<Self, GeneratorError> {
        let name = column.name().to_owned();
        let domain = match column.column_type() {
            ColumnType::Int => {
                let mut domain = IntDomain::new();
                for constraint in column.constraints() {
                    let Constraint::Int(constraint) = *constraint else {
                        return Err(GeneratorError::ConstraintTypeMismatch {
                            column: name,
                            column_type: ColumnType::Int,
                            constraint_type: constraint.column_type(),
                        });
                    };
                    domain
                        .apply(constraint)
                        .map_err(|source| GeneratorError::Solver {
                            column: column.name().to_owned(),
                            source,
                        })?;
                }
                GenerationDomain::Int(domain)
            }
            ColumnType::Timestamp => {
                let mut domain = TimestampDomain::new();
                for constraint in column.constraints() {
                    let Constraint::Timestamp(constraint) = *constraint else {
                        return Err(GeneratorError::ConstraintTypeMismatch {
                            column: name,
                            column_type: ColumnType::Timestamp,
                            constraint_type: constraint.column_type(),
                        });
                    };
                    domain
                        .apply(constraint)
                        .map_err(|source| GeneratorError::Solver {
                            column: column.name().to_owned(),
                            source,
                        })?;
                }
                GenerationDomain::Timestamp(domain)
            }
            ColumnType::Bool => {
                let mut domain = BoolDomain::new();
                for constraint in column.constraints() {
                    let Constraint::Bool(constraint) = *constraint else {
                        return Err(GeneratorError::ConstraintTypeMismatch {
                            column: name,
                            column_type: ColumnType::Bool,
                            constraint_type: constraint.column_type(),
                        });
                    };
                    domain
                        .apply(constraint)
                        .map_err(|source| GeneratorError::Solver {
                            column: column.name().to_owned(),
                            source,
                        })?;
                }
                GenerationDomain::Bool(domain)
            }
            ColumnType::String => {
                return Err(GeneratorError::UnsupportedColumnType {
                    column: name,
                    column_type: ColumnType::String,
                });
            }
        };

        Ok(Self { name, domain })
    }

    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Result<TableValue, GeneratorError> {
        match &self.domain {
            GenerationDomain::Int(domain) => {
                let index = sample_index(rng, domain.value_count(), &self.name)?;
                let value = domain
                    .value_at(index)
                    .map_err(|source| GeneratorError::Solver {
                        column: self.name.clone(),
                        source,
                    })?;
                Ok(TableValue::Int(value))
            }
            GenerationDomain::Timestamp(domain) => {
                let index = sample_index(rng, domain.value_count(), &self.name)?;
                let value =
                    domain
                        .value_at(index)
                        .map_err(|source| GeneratorError::Solver {
                            column: self.name.clone(),
                            source,
                        })?;
                Ok(TableValue::Timestamp(value))
            }
            GenerationDomain::Bool(domain) => Ok(TableValue::Bool(domain.value())),
        }
    }
}

#[derive(Debug, Clone)]
enum GenerationDomain {
    Int(IntDomain),
    Timestamp(TimestampDomain),
    Bool(BoolDomain),
}

fn sample_index<R: Rng + ?Sized>(
    rng: &mut R,
    value_count: usize,
    column: &str,
) -> Result<usize, GeneratorError> {
    if value_count == 0 {
        return Err(GeneratorError::EmptyDomain {
            column: column.to_owned(),
        });
    }

    let upper = u64::try_from(value_count).map_err(|_| GeneratorError::DomainTooLarge {
        column: column.to_owned(),
        value_count,
    })?;
    let zone = u64::MAX - (u64::MAX % upper);

    loop {
        let candidate = rng.next_u64();
        if candidate < zone {
            let index = candidate % upper;
            return usize::try_from(index).map_err(|_| GeneratorError::DomainTooLarge {
                column: column.to_owned(),
                value_count,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        BoolConstraint, Column, ColumnType, Constraint, IntConstraint, SolverError, Table,
        TimestampConstraint,
    };

    use super::{Generator, GeneratorError};

    fn constrained_column(
        name: &str,
        column_type: ColumnType,
        constraints: &[Constraint],
    ) -> Column {
        let mut column = Column::new(name, column_type);
        for constraint in constraints {
            column
                .add_constraint(*constraint)
                .expect("test constraint should match column type");
        }
        column
    }

    #[test]
    fn generate_ints() {
        let schema = vec![constrained_column(
            "col_a",
            ColumnType::Int,
            &[
                Constraint::Int(IntConstraint::NotEqual(10)),
                Constraint::Int(IntConstraint::GreaterThan(3)),
                Constraint::Int(IntConstraint::LessThan(100)),
            ],
        )];
        let mut first = Table::new(schema.clone(), 13).expect("test table should be valid");
        let mut second = Table::new(schema, 13).expect("test table should be valid");
        let generator = Generator::new();

        generator
            .generate(&mut first, 42)
            .expect("integer generation should succeed");
        generator
            .generate(&mut second, 42)
            .expect("integer generation should be deterministic");
        first.build_ints();
        second.build_ints();

        let first_values = first
            .get_ints("col_a")
            .expect("integer column should be readable")
            .expect("integer column should be built");
        let second_values = second
            .get_ints("col_a")
            .expect("integer column should be readable")
            .expect("integer column should be built");

        assert_eq!(first_values, second_values);
        assert_eq!(first_values.len(), 13);
        assert!(
            first_values
                .iter()
                .all(|value| *value > 3 && *value < 100 && *value != 10)
        );
    }

    #[test]
    fn generate_timestamps() {
        let excluded = 1_371_471_500;
        let schema = vec![constrained_column(
            "created_at",
            ColumnType::Timestamp,
            &[
                Constraint::Timestamp(TimestampConstraint::NotEqual(excluded)),
                Constraint::Timestamp(TimestampConstraint::GreaterThan(1_371_471_000)),
                Constraint::Timestamp(TimestampConstraint::LessThan(1_371_472_000)),
            ],
        )];
        let mut table = Table::new(schema, 13).expect("test table should be valid");

        Generator::new()
            .generate(&mut table, 42)
            .expect("timestamp generation should succeed");
        table.build_timestamps();

        let values = table
            .get_timestamps("created_at")
            .expect("timestamp column should be readable")
            .expect("timestamp column should be built");

        assert_eq!(values.len(), 13);
        assert!(values.iter().all(|value| {
            let seconds = value.timestamp();
            seconds > 1_371_471_000 && seconds < 1_371_472_000 && seconds != i64::from(excluded)
        }));
    }

    #[test]
    fn generate_bools() {
        let schema = vec![
            constrained_column(
                "enabled",
                ColumnType::Bool,
                &[Constraint::Bool(BoolConstraint::IsTrue)],
            ),
            constrained_column(
                "disabled",
                ColumnType::Bool,
                &[Constraint::Bool(BoolConstraint::IsFalse)],
            ),
        ];
        let mut table = Table::new(schema, 13).expect("test table should be valid");

        Generator::new()
            .generate(&mut table, 42)
            .expect("boolean generation should succeed");
        table.build_bools();

        let enabled = table
            .get_bools("enabled")
            .expect("boolean column should be readable")
            .expect("boolean column should be built");
        let disabled = table
            .get_bools("disabled")
            .expect("boolean column should be readable")
            .expect("boolean column should be built");

        assert_eq!(enabled.len(), 13);
        assert_eq!(disabled.len(), 13);
        assert!(enabled.iter().all(|value| *value));
        assert!(disabled.iter().all(|value| !*value));
    }

    #[test]
    fn unsupported_column_type_returns_error() {
        let mut table =
            Table::new(vec![Column::new("name", ColumnType::String)], 13).expect("valid table");

        let error = Generator::new()
            .generate(&mut table, 42)
            .expect_err("unsupported string generation should fail");

        assert_eq!(
            error,
            GeneratorError::UnsupportedColumnType {
                column: "name".to_owned(),
                column_type: ColumnType::String,
            }
        );
        table.build_strings();
        assert_eq!(
            table
                .get_strings("name")
                .expect("string column should be readable"),
            Some(Vec::new())
        );
    }

    #[test]
    fn contradictory_constraints_return_error_before_writing_rows() {
        let schema = vec![
            constrained_column(
                "valid",
                ColumnType::Int,
                &[Constraint::Int(IntConstraint::Equal(7))],
            ),
            constrained_column(
                "invalid",
                ColumnType::Int,
                &[
                    Constraint::Int(IntConstraint::Equal(1)),
                    Constraint::Int(IntConstraint::NotEqual(1)),
                ],
            ),
        ];
        let mut table = Table::new(schema, 13).expect("test table should be valid");

        let error = Generator::new()
            .generate(&mut table, 42)
            .expect_err("contradictory constraints should fail");

        assert_eq!(
            error,
            GeneratorError::Solver {
                column: "invalid".to_owned(),
                source: SolverError::UnsatisfiableDomain,
            }
        );

        table.build_ints();
        assert_eq!(
            table
                .get_ints("valid")
                .expect("integer column should be readable"),
            Some(Vec::new())
        );
    }
}
