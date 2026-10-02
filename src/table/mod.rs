//! Arrow-backed columnar table storage.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

use arrow_array::builder::{
    BooleanBuilder, Int32Builder, StringBuilder, TimestampMicrosecondBuilder,
};
use arrow_array::{
    Array, ArrayRef, BooleanArray, Int32Array, StringArray, TimestampMicrosecondArray,
};
use arrow_ord::sort::sort;
use arrow_schema::{DataType, Field, Schema, TimeUnit};
use chrono::{DateTime, Utc};

use crate::types::{Column, ColumnType};

#[cfg(test)]
mod tests;

/// Dimensions declared for a table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dim {
    rows: usize,
    cols: usize,
}

impl Dim {
    /// Returns the declared row count.
    pub const fn rows(self) -> usize {
        self.rows
    }

    /// Returns the number of schema columns.
    pub const fn cols(self) -> usize {
        self.cols
    }
}

/// A value that can be appended to a table column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableValue {
    /// Signed 32-bit integer value.
    Int(i32),
    /// UTC timestamp value.
    Timestamp(DateTime<Utc>),
    /// Boolean value.
    Bool(bool),
    /// UTF-8 string value.
    String(String),
}

impl TableValue {
    /// Returns the column type represented by this value.
    pub const fn column_type(&self) -> ColumnType {
        match self {
            Self::Int(_) => ColumnType::Int,
            Self::Timestamp(_) => ColumnType::Timestamp,
            Self::Bool(_) => ColumnType::Bool,
            Self::String(_) => ColumnType::String,
        }
    }
}

/// Errors returned by table construction and operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// The schema contains the same column name more than once.
    DuplicateColumn {
        /// Duplicated column name.
        column: String,
    },
    /// No schema column has the requested name.
    UnknownColumn {
        /// Requested column name.
        column: String,
    },
    /// A value or typed getter does not match the column's declared type.
    ColumnTypeMismatch {
        /// Column name.
        column: String,
        /// Type declared by the schema.
        expected: ColumnType,
        /// Type supplied by the operation.
        actual: ColumnType,
    },
    /// Arrow returned an array type that does not match the schema.
    InvalidArrowArray {
        /// Column name.
        column: String,
    },
    /// A built array unexpectedly contains a null value.
    UnexpectedNull {
        /// Column name.
        column: String,
    },
    /// A stored microsecond timestamp cannot be represented as a UTC timestamp.
    TimestampOutOfRange {
        /// Column name.
        column: String,
        /// Stored Unix microseconds.
        micros: i64,
    },
    /// An Arrow compute operation failed.
    Arrow {
        /// Arrow error message.
        message: String,
    },
}

impl fmt::Display for TableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateColumn { column } => write!(formatter, "duplicate column {column:?}"),
            Self::UnknownColumn { column } => write!(formatter, "unknown column {column:?}"),
            Self::ColumnTypeMismatch {
                column,
                expected,
                actual,
            } => write!(
                formatter,
                "column {column:?} expects {expected}, got {actual}"
            ),
            Self::InvalidArrowArray { column } => {
                write!(formatter, "invalid Arrow array for column {column:?}")
            }
            Self::UnexpectedNull { column } => {
                write!(formatter, "unexpected null in column {column:?}")
            }
            Self::TimestampOutOfRange { column, micros } => write!(
                formatter,
                "timestamp {micros} microseconds in column {column:?} is outside supported range"
            ),
            Self::Arrow { message } => write!(formatter, "arrow operation failed: {message}"),
        }
    }
}

impl Error for TableError {}

enum ColumnBuilder {
    Int(Int32Builder),
    Timestamp(TimestampMicrosecondBuilder),
    Bool(BooleanBuilder),
    String(StringBuilder),
}

impl ColumnBuilder {
    fn new(column_type: ColumnType) -> Self {
        match column_type {
            ColumnType::Int => Self::Int(Int32Builder::new()),
            ColumnType::Timestamp => Self::Timestamp(TimestampMicrosecondBuilder::new()),
            ColumnType::Bool => Self::Bool(BooleanBuilder::new()),
            ColumnType::String => Self::String(StringBuilder::new()),
        }
    }
}

struct ColumnStorage {
    column_type: ColumnType,
    builder: ColumnBuilder,
    array: Option<ArrayRef>,
}

impl ColumnStorage {
    fn new(column_type: ColumnType) -> Self {
        Self {
            column_type,
            builder: ColumnBuilder::new(column_type),
            array: None,
        }
    }

    fn wipe(&mut self) {
        self.builder = ColumnBuilder::new(self.column_type);
        self.array = None;
    }
}

/// Arrow-backed table with one builder and optional finalized array per schema column.
pub struct Table {
    schema: Vec<Column>,
    arrow_schema: Schema,
    dim: Dim,
    columns: BTreeMap<String, ColumnStorage>,
}

impl Table {
    /// Creates an empty table with builders for every schema column.
    pub fn new(schema: Vec<Column>, rows: usize) -> Result<Self, TableError> {
        let mut columns = BTreeMap::new();
        for column in &schema {
            if columns
                .insert(
                    column.name().to_owned(),
                    ColumnStorage::new(column.column_type()),
                )
                .is_some()
            {
                return Err(TableError::DuplicateColumn {
                    column: column.name().to_owned(),
                });
            }
        }

        let fields = schema
            .iter()
            .map(|column| Field::new(column.name(), arrow_data_type(column.column_type()), false))
            .collect::<Vec<_>>();

        Ok(Self {
            dim: Dim {
                rows,
                cols: schema.len(),
            },
            schema,
            arrow_schema: Schema::new(fields),
            columns,
        })
    }

    /// Returns the project schema in declaration order.
    pub fn schema(&self) -> &[Column] {
        &self.schema
    }

    /// Returns the Arrow schema derived from the project schema.
    pub fn arrow_schema(&self) -> &Schema {
        &self.arrow_schema
    }

    /// Returns the declared table dimensions.
    pub const fn dim(&self) -> Dim {
        self.dim
    }

    /// Returns the declared type for one column.
    pub fn column_type(&self, column: &str) -> Result<ColumnType, TableError> {
        self.columns
            .get(column)
            .map(|storage| storage.column_type)
            .ok_or_else(|| TableError::UnknownColumn {
                column: column.to_owned(),
            })
    }

    /// Appends one value to the named column after validating its type.
    pub fn append(&mut self, column: &str, value: TableValue) -> Result<(), TableError> {
        let actual = value.column_type();
        let storage = self
            .columns
            .get_mut(column)
            .ok_or_else(|| TableError::UnknownColumn {
                column: column.to_owned(),
            })?;

        if storage.column_type != actual {
            return Err(TableError::ColumnTypeMismatch {
                column: column.to_owned(),
                expected: storage.column_type,
                actual,
            });
        }

        match (&mut storage.builder, value) {
            (ColumnBuilder::Int(builder), TableValue::Int(value)) => {
                builder.append_value(value);
            }
            (ColumnBuilder::Timestamp(builder), TableValue::Timestamp(value)) => {
                builder.append_value(value.timestamp_micros());
            }
            (ColumnBuilder::Bool(builder), TableValue::Bool(value)) => {
                builder.append_value(value);
            }
            (ColumnBuilder::String(builder), TableValue::String(value)) => {
                builder.append_value(&value);
            }
            _ => {
                return Err(TableError::ColumnTypeMismatch {
                    column: column.to_owned(),
                    expected: storage.column_type,
                    actual,
                });
            }
        }

        Ok(())
    }

    /// Finalizes every integer builder into an Arrow array.
    pub fn build_ints(&mut self) {
        for storage in self.columns.values_mut() {
            if let ColumnBuilder::Int(builder) = &mut storage.builder {
                storage.array = Some(Arc::new(builder.finish()));
            }
        }
    }

    /// Finalizes every timestamp builder into an Arrow array.
    pub fn build_timestamps(&mut self) {
        for storage in self.columns.values_mut() {
            if let ColumnBuilder::Timestamp(builder) = &mut storage.builder {
                storage.array = Some(Arc::new(builder.finish()));
            }
        }
    }

    /// Finalizes every boolean builder into an Arrow array.
    pub fn build_bools(&mut self) {
        for storage in self.columns.values_mut() {
            if let ColumnBuilder::Bool(builder) = &mut storage.builder {
                storage.array = Some(Arc::new(builder.finish()));
            }
        }
    }

    /// Finalizes every string builder into an Arrow array.
    pub fn build_strings(&mut self) {
        for storage in self.columns.values_mut() {
            if let ColumnBuilder::String(builder) = &mut storage.builder {
                storage.array = Some(Arc::new(builder.finish()));
            }
        }
    }

    /// Finalizes builders for every supported column type.
    pub fn build_all(&mut self) {
        self.build_ints();
        self.build_timestamps();
        self.build_bools();
        self.build_strings();
    }

    /// Returns the built integer values for one column, or None before finalization.
    pub fn get_ints(&self, column: &str) -> Result<Option<Vec<i32>>, TableError> {
        let array = self.typed_array::<Int32Array>(column, ColumnType::Int)?;
        array
            .map(|array| collect_non_null(array.iter(), column))
            .transpose()
    }

    /// Returns all integer columns keyed by schema column name.
    pub fn get_all_ints(&self) -> Result<BTreeMap<String, Option<Vec<i32>>>, TableError> {
        let mut result = BTreeMap::new();
        for column in &self.schema {
            if column.column_type() == ColumnType::Int {
                result.insert(column.name().to_owned(), self.get_ints(column.name())?);
            }
        }
        Ok(result)
    }

    /// Returns the built timestamp values for one column, or None before finalization.
    pub fn get_timestamps(&self, column: &str) -> Result<Option<Vec<DateTime<Utc>>>, TableError> {
        let array = self.typed_array::<TimestampMicrosecondArray>(column, ColumnType::Timestamp)?;
        array
            .map(|array| {
                array
                    .iter()
                    .map(|value| {
                        let micros = value.ok_or_else(|| TableError::UnexpectedNull {
                            column: column.to_owned(),
                        })?;
                        datetime_from_micros(column, micros)
                    })
                    .collect()
            })
            .transpose()
    }

    /// Returns all timestamp columns keyed by schema column name.
    pub fn get_all_timestamps(
        &self,
    ) -> Result<BTreeMap<String, Option<Vec<DateTime<Utc>>>>, TableError> {
        let mut result = BTreeMap::new();
        for column in &self.schema {
            if column.column_type() == ColumnType::Timestamp {
                result.insert(
                    column.name().to_owned(),
                    self.get_timestamps(column.name())?,
                );
            }
        }
        Ok(result)
    }

    /// Returns the built boolean values for one column, or None before finalization.
    pub fn get_bools(&self, column: &str) -> Result<Option<Vec<bool>>, TableError> {
        let array = self.typed_array::<BooleanArray>(column, ColumnType::Bool)?;
        array
            .map(|array| collect_non_null(array.iter(), column))
            .transpose()
    }

    /// Returns all boolean columns keyed by schema column name.
    pub fn get_all_bools(&self) -> Result<BTreeMap<String, Option<Vec<bool>>>, TableError> {
        let mut result = BTreeMap::new();
        for column in &self.schema {
            if column.column_type() == ColumnType::Bool {
                result.insert(column.name().to_owned(), self.get_bools(column.name())?);
            }
        }
        Ok(result)
    }

    /// Returns the built string values for one column, or None before finalization.
    pub fn get_strings(&self, column: &str) -> Result<Option<Vec<String>>, TableError> {
        let array = self.typed_array::<StringArray>(column, ColumnType::String)?;
        array
            .map(|array| {
                array
                    .iter()
                    .map(|value| {
                        value
                            .map(str::to_owned)
                            .ok_or_else(|| TableError::UnexpectedNull {
                                column: column.to_owned(),
                            })
                    })
                    .collect()
            })
            .transpose()
    }

    /// Returns all string columns keyed by schema column name.
    pub fn get_all_strings(&self) -> Result<BTreeMap<String, Option<Vec<String>>>, TableError> {
        let mut result = BTreeMap::new();
        for column in &self.schema {
            if column.column_type() == ColumnType::String {
                result.insert(column.name().to_owned(), self.get_strings(column.name())?);
            }
        }
        Ok(result)
    }

    /// Sorts every built integer column in ascending order.
    pub fn sort_ints(&mut self) -> Result<(), TableError> {
        self.sort_built_type(ColumnType::Int)
    }

    /// Sorts every built timestamp column in ascending order.
    pub fn sort_timestamps(&mut self) -> Result<(), TableError> {
        self.sort_built_type(ColumnType::Timestamp)
    }

    /// Clears built arrays and resets every builder for reuse.
    pub fn wipe(&mut self) {
        for storage in self.columns.values_mut() {
            storage.wipe();
        }
    }

    fn typed_array<T>(&self, column: &str, requested: ColumnType) -> Result<Option<&T>, TableError>
    where
        T: Array + 'static,
    {
        let storage = self
            .columns
            .get(column)
            .ok_or_else(|| TableError::UnknownColumn {
                column: column.to_owned(),
            })?;

        if storage.column_type != requested {
            return Err(TableError::ColumnTypeMismatch {
                column: column.to_owned(),
                expected: storage.column_type,
                actual: requested,
            });
        }

        storage
            .array
            .as_deref()
            .map(|array| {
                array
                    .as_any()
                    .downcast_ref::<T>()
                    .ok_or_else(|| TableError::InvalidArrowArray {
                        column: column.to_owned(),
                    })
            })
            .transpose()
    }

    fn sort_built_type(&mut self, column_type: ColumnType) -> Result<(), TableError> {
        for storage in self.columns.values_mut() {
            if storage.column_type != column_type {
                continue;
            }

            let Some(array) = &storage.array else {
                continue;
            };
            let sorted = sort(array.as_ref(), None).map_err(|error| TableError::Arrow {
                message: error.to_string(),
            })?;
            storage.array = Some(sorted);
        }
        Ok(())
    }
}

fn arrow_data_type(column_type: ColumnType) -> DataType {
    match column_type {
        ColumnType::Int => DataType::Int32,
        ColumnType::Timestamp => DataType::Timestamp(TimeUnit::Microsecond, None),
        ColumnType::Bool => DataType::Boolean,
        ColumnType::String => DataType::Utf8,
    }
}

fn collect_non_null<T, I>(values: I, column: &str) -> Result<Vec<T>, TableError>
where
    I: IntoIterator<Item = Option<T>>,
{
    values
        .into_iter()
        .map(|value| {
            value.ok_or_else(|| TableError::UnexpectedNull {
                column: column.to_owned(),
            })
        })
        .collect()
}

fn datetime_from_micros(column: &str, micros: i64) -> Result<DateTime<Utc>, TableError> {
    let seconds = micros.div_euclid(1_000_000);
    let micros_remainder = u32::try_from(micros.rem_euclid(1_000_000)).map_err(|_| {
        TableError::TimestampOutOfRange {
            column: column.to_owned(),
            micros,
        }
    })?;
    let nanos =
        micros_remainder
            .checked_mul(1_000)
            .ok_or_else(|| TableError::TimestampOutOfRange {
                column: column.to_owned(),
                micros,
            })?;

    DateTime::<Utc>::from_timestamp(seconds, nanos).ok_or_else(|| TableError::TimestampOutOfRange {
        column: column.to_owned(),
        micros,
    })
}
