//! DuckDB materialization and workload execution boundary.
//!
//! This module translates already-generated Arrow values into DuckDB storage. It never analyzes
//! SQL semantics: all generation constraints remain owned by SQL Semantic Protocol.

use std::error::Error;
use std::fmt;
use std::path::Path;

use arrow_array::{
    Array, BinaryArray, BooleanArray, Date32Array, Decimal128Array, FixedSizeBinaryArray,
    FixedSizeListArray, Float32Array, Float64Array, Int8Array, Int16Array, Int32Array, Int64Array,
    ListArray, MapArray, StringArray, StructArray, Time64MicrosecondArray, Time64NanosecondArray,
    TimestampMicrosecondArray, TimestampNanosecondArray, UInt8Array, UInt16Array, UInt32Array,
    UInt64Array,
};
use arrow_schema::{DataType as ArrowDataType, TimeUnit as ArrowTimeUnit};
use duckdb::types::{Decimal, OrderedMap, TimeUnit, Value};
use duckdb::{AccessMode, Config, Connection, params_from_iter};
use sql_semantic_protocol::DataType as ProtocolDataType;

use crate::{
    GeneratedData, QueryResult, ResultColumn, ResultOrdering, Table, TestCase, TestCaseError,
    VerificationError,
};

/// DuckDB execution failures isolated from generation and protocol semantics.
#[derive(Debug)]
pub enum DuckDbExecutionError {
    /// DuckDB rejected a materialization or workload operation.
    DuckDb {
        /// Backend error message.
        message: String,
    },
    /// A generated relation identity cannot be represented as a DuckDB table identity.
    InvalidRelation {
        /// Relation identity supplied by the protocol.
        relation: String,
    },
    /// A generated Arrow datatype has no lossless DuckDB materialization mapping.
    UnsupportedArrowType {
        /// Arrow datatype description.
        data_type: String,
    },
    /// A protocol datatype has no lossless DuckDB materialization mapping.
    UnsupportedProtocolType {
        /// Canonical protocol datatype kind.
        data_type: String,
    },
    /// A table column has not been finalized into an Arrow array.
    UnbuiltColumn {
        /// Relation identity.
        relation: String,
        /// Column name.
        column: String,
    },
    /// A dynamic Arrow value did not match its declared datatype.
    InvalidArrowValue {
        /// Arrow datatype description.
        data_type: String,
    },
    /// An Arrow-backed table operation failed while materializing data.
    Table {
        /// Table error message.
        message: String,
    },
    /// A test-case snapshot or observed query result is invalid.
    TestCase {
        /// Validation failure.
        message: String,
    },
    /// Current workload output differs from the explicit approval contract.
    Verification(VerificationError),
}

impl fmt::Display for DuckDbExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuckDb { message } => write!(formatter, "DuckDB operation failed: {message}"),
            Self::InvalidRelation { relation } => {
                write!(
                    formatter,
                    "relation {relation:?} is not a valid DuckDB table identity"
                )
            }
            Self::UnsupportedArrowType { data_type } => {
                write!(
                    formatter,
                    "Arrow datatype {data_type} cannot be materialized in DuckDB"
                )
            }
            Self::UnsupportedProtocolType { data_type } => {
                write!(
                    formatter,
                    "protocol datatype {data_type} cannot be materialized in DuckDB"
                )
            }
            Self::UnbuiltColumn { relation, column } => write!(
                formatter,
                "relation {relation:?} column {column:?} has no finalized Arrow array"
            ),
            Self::InvalidArrowValue { data_type } => write!(
                formatter,
                "Arrow array value does not match declared datatype {data_type}"
            ),
            Self::Table { message } => write!(formatter, "table operation failed: {message}"),
            Self::TestCase { message } => write!(formatter, "invalid test-case result: {message}"),
            Self::Verification(error) => error.fmt(formatter),
        }
    }
}

impl Error for DuckDbExecutionError {}

impl From<duckdb::Error> for DuckDbExecutionError {
    fn from(error: duckdb::Error) -> Self {
        Self::DuckDb {
            message: error.to_string(),
        }
    }
}

impl From<crate::TableError> for DuckDbExecutionError {
    fn from(error: crate::TableError) -> Self {
        Self::Table {
            message: error.to_string(),
        }
    }
}

impl From<TestCaseError> for DuckDbExecutionError {
    fn from(error: TestCaseError) -> Self {
        Self::TestCase {
            message: error.to_string(),
        }
    }
}

impl From<VerificationError> for DuckDbExecutionError {
    fn from(error: VerificationError) -> Self {
        Self::Verification(error)
    }
}

/// Owns one DuckDB connection used only for materialization and workload execution.
pub struct DuckDbExecutor {
    connection: Connection,
}

impl DuckDbExecutor {
    /// Opens a fresh in-memory DuckDB database.
    pub fn in_memory() -> Result<Self, DuckDbExecutionError> {
        Ok(Self {
            connection: Connection::open_in_memory()?,
        })
    }

    /// Creates or opens an explicit writable DuckDB database output path.
    pub fn create(path: impl AsRef<Path>) -> Result<Self, DuckDbExecutionError> {
        Ok(Self {
            connection: Connection::open(path)?,
        })
    }

    /// Opens an existing DuckDB database without permitting writes.
    pub fn open_read_only(path: impl AsRef<Path>) -> Result<Self, DuckDbExecutionError> {
        let config = Config::default().access_mode(AccessMode::ReadOnly)?;
        Ok(Self {
            connection: Connection::open_with_flags(path, config)?,
        })
    }

    /// Materializes every generated relation using exact protocol logical types where available.
    pub fn materialize(&self, generated: &GeneratedData) -> Result<(), DuckDbExecutionError> {
        for (relation, table) in generated.tables() {
            self.materialize_table(relation, table)?;
        }
        Ok(())
    }

    /// Materializes one CLI-exported Parquet relation into DuckDB.
    pub fn materialize_parquet(
        &self,
        relation: &str,
        path: impl AsRef<Path>,
    ) -> Result<(), DuckDbExecutionError> {
        let identity = QualifiedRelation::parse(relation)?;
        if let Some(schema) = &identity.schema {
            self.connection.execute_batch(&format!(
                "CREATE SCHEMA IF NOT EXISTS {}",
                quote_identifier(schema)
            ))?;
        }

        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| DuckDbExecutionError::DuckDb {
                message: "Parquet path is not valid UTF-8".to_owned(),
            })?;
        let qualified = identity.sql_name();
        self.connection.execute_batch(&format!(
            "DROP TABLE IF EXISTS {qualified}; \
             CREATE TABLE {qualified} AS SELECT * FROM read_parquet({})",
            quote_literal(path)
        ))?;
        Ok(())
    }

    /// Executes an ordered SQL workload and captures the final statement result.
    ///
    /// DuckDB prepares the complete input and executes preceding statements before yielding the
    /// final statement result. This keeps workload ordering explicit without splitting SQL text.
    pub fn execute(&self, workload: &str) -> Result<QueryResult, DuckDbExecutionError> {
        let mut statement = self.connection.prepare(workload)?;
        let mut rows = statement.query([])?;
        let statement = rows.as_ref().ok_or_else(|| DuckDbExecutionError::DuckDb {
            message: "workload did not expose a prepared final statement".to_owned(),
        })?;

        let column_count = statement.column_count();
        let mut columns = Vec::with_capacity(column_count);
        for index in 0..column_count {
            columns.push(ResultColumn::new(
                statement.column_name(index)?.clone(),
                format!("{:?}", statement.column_logical_type(index)),
            )?);
        }

        let mut result_rows = Vec::new();
        while let Some(row) = rows.next()? {
            let mut result_row = Vec::with_capacity(column_count);
            for index in 0..column_count {
                let value: Value = row.get(index)?;
                result_row.push(canonical_value(&value)?);
            }
            result_rows.push(result_row);
        }

        QueryResult::new(columns, result_rows).map_err(Into::into)
    }

    /// Executes a workload and explicitly replaces the approved expected result.
    pub fn approve_workload(
        &self,
        test_case: &mut TestCase,
        workload: &str,
        ordering: ResultOrdering,
    ) -> Result<(), DuckDbExecutionError> {
        let result = self.execute(workload)?;
        test_case.approve_result(result, ordering);
        Ok(())
    }

    /// Executes a workload and verifies it without modifying the approved result.
    pub fn verify_workload(
        &self,
        test_case: &TestCase,
        workload: &str,
    ) -> Result<(), DuckDbExecutionError> {
        let current = self.execute(workload)?;
        test_case.verify_result(&current)?;
        Ok(())
    }

    fn materialize_table(&self, relation: &str, table: &Table) -> Result<(), DuckDbExecutionError> {
        let identity = QualifiedRelation::parse(relation)?;
        if let Some(schema) = &identity.schema {
            self.connection.execute_batch(&format!(
                "CREATE SCHEMA IF NOT EXISTS {}",
                quote_identifier(schema)
            ))?;
        }

        let qualified = identity.sql_name();
        self.connection
            .execute_batch(&format!("DROP TABLE IF EXISTS {qualified}"))?;

        let fields = table.arrow_schema().fields();
        let protocol_schema = table.protocol_schema();
        if protocol_schema.is_some_and(|schema| schema.len() != fields.len()) {
            return Err(DuckDbExecutionError::InvalidArrowValue {
                data_type: "protocol schema column count differs from Arrow schema".to_owned(),
            });
        }

        let mut columns = Vec::with_capacity(fields.len());
        for (index, field) in fields.iter().enumerate() {
            let data_type = match protocol_schema {
                Some(schema) => protocol_type_sql(schema[index].data_type())?,
                None => arrow_type_sql(field.data_type())?,
            };
            columns.push(format!("{} {data_type}", quote_identifier(field.name())));
        }
        self.connection.execute_batch(&format!(
            "CREATE TABLE {qualified} ({})",
            columns.join(", ")
        ))?;

        if table.dim().rows() == 0 {
            return Ok(());
        }

        for row_index in 0..table.dim().rows() {
            let mut expressions = Vec::with_capacity(fields.len());
            let mut parameters = Vec::with_capacity(fields.len());
            for (column_index, field) in fields.iter().enumerate() {
                let array = table.array(field.name())?.ok_or_else(|| {
                    DuckDbExecutionError::UnbuiltColumn {
                        relation: relation.to_owned(),
                        column: field.name().to_owned(),
                    }
                })?;
                let protocol_type = protocol_schema.map(|schema| schema[column_index].data_type());
                let value = arrow_value(array, row_index, protocol_type)?;
                expressions.push(bind_expression(value, &mut parameters)?);
            }
            self.connection.execute(
                &format!(
                    "INSERT INTO {qualified} VALUES ({})",
                    expressions.join(", ")
                ),
                params_from_iter(parameters),
            )?;
        }

        Ok(())
    }
}

struct QualifiedRelation {
    schema: Option<String>,
    table: String,
}

impl QualifiedRelation {
    fn parse(relation: &str) -> Result<Self, DuckDbExecutionError> {
        let parts = relation.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            [table] if !table.is_empty() => Ok(Self {
                schema: None,
                table: (*table).to_owned(),
            }),
            [schema, table] if !schema.is_empty() && !table.is_empty() => Ok(Self {
                schema: Some((*schema).to_owned()),
                table: (*table).to_owned(),
            }),
            _ => Err(DuckDbExecutionError::InvalidRelation {
                relation: relation.to_owned(),
            }),
        }
    }

    fn sql_name(&self) -> String {
        match &self.schema {
            Some(schema) => format!(
                "{}.{}",
                quote_identifier(schema),
                quote_identifier(&self.table)
            ),
            None => quote_identifier(&self.table),
        }
    }
}

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn protocol_type_sql(data_type: &ProtocolDataType) -> Result<String, DuckDbExecutionError> {
    let sql = match data_type {
        ProtocolDataType::Nullable(inner) => return protocol_type_sql(inner),
        ProtocolDataType::Boolean => "BOOLEAN".to_owned(),
        ProtocolDataType::SignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => "TINYINT".to_owned(),
            9..=16 => "SMALLINT".to_owned(),
            17..=32 => "INTEGER".to_owned(),
            33..=64 => "BIGINT".to_owned(),
            other => return unsupported_protocol(format!("signed_integer({other})")),
        },
        ProtocolDataType::UnsignedInteger { bits } => match bits.unwrap_or(64) {
            0..=8 => "UTINYINT".to_owned(),
            9..=16 => "USMALLINT".to_owned(),
            17..=32 => "UINTEGER".to_owned(),
            33..=64 => "UBIGINT".to_owned(),
            other => return unsupported_protocol(format!("unsigned_integer({other})")),
        },
        ProtocolDataType::Decimal { precision, scale } => format!(
            "DECIMAL({}, {})",
            precision.unwrap_or(38),
            scale.unwrap_or(0)
        ),
        ProtocolDataType::FloatingPoint { bits } if bits.unwrap_or(64) <= 32 => "FLOAT".to_owned(),
        ProtocolDataType::FloatingPoint { bits } if bits.unwrap_or(64) <= 64 => "DOUBLE".to_owned(),
        ProtocolDataType::FloatingPoint { bits } => {
            return unsupported_protocol(format!("floating_point({})", bits.unwrap_or_default()));
        }
        ProtocolDataType::String { .. } | ProtocolDataType::Set { .. } => "VARCHAR".to_owned(),
        ProtocolDataType::Binary { .. } => "BLOB".to_owned(),
        ProtocolDataType::Date => "DATE".to_owned(),
        ProtocolDataType::Time { precision } if precision.is_some_and(|value| value > 6) => {
            "TIME_NS".to_owned()
        }
        ProtocolDataType::Time { .. } => "TIME".to_owned(),
        ProtocolDataType::Timestamp { precision } if precision.is_some_and(|value| value > 6) => {
            "TIMESTAMP_NS".to_owned()
        }
        ProtocolDataType::Timestamp { .. } => "TIMESTAMP".to_owned(),
        ProtocolDataType::Uuid => "UUID".to_owned(),
        ProtocolDataType::Json => "JSON".to_owned(),
        ProtocolDataType::Array { element, length } => {
            let element = element.as_deref().ok_or_else(|| {
                DuckDbExecutionError::UnsupportedProtocolType {
                    data_type: "array without element datatype".to_owned(),
                }
            })?;
            let element = protocol_type_sql(element)?;
            match length {
                Some(length) => format!("{element}[{length}]"),
                None => format!("{element}[]"),
            }
        }
        ProtocolDataType::Map { key, value } => {
            format!(
                "MAP({}, {})",
                protocol_type_sql(key)?,
                protocol_type_sql(value)?
            )
        }
        ProtocolDataType::Struct { fields } => {
            let fields = fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let name = field
                        .name()
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("field_{index}"));
                    Ok(format!(
                        "{} {}",
                        quote_identifier(&name),
                        protocol_type_sql(field.data_type())?
                    ))
                })
                .collect::<Result<Vec<_>, DuckDbExecutionError>>()?;
            format!("STRUCT({})", fields.join(", "))
        }
        ProtocolDataType::Enum { values } => {
            let values = values
                .iter()
                .map(|value| quote_literal(value.name()))
                .collect::<Vec<_>>();
            format!("ENUM ({})", values.join(", "))
        }
        unsupported => return unsupported_protocol(unsupported.kind().to_owned()),
    };
    Ok(sql)
}

fn unsupported_protocol<T>(data_type: String) -> Result<T, DuckDbExecutionError> {
    Err(DuckDbExecutionError::UnsupportedProtocolType { data_type })
}

fn arrow_type_sql(data_type: &ArrowDataType) -> Result<String, DuckDbExecutionError> {
    let sql = match data_type {
        ArrowDataType::Boolean => "BOOLEAN".to_owned(),
        ArrowDataType::Int8 => "TINYINT".to_owned(),
        ArrowDataType::Int16 => "SMALLINT".to_owned(),
        ArrowDataType::Int32 => "INTEGER".to_owned(),
        ArrowDataType::Int64 => "BIGINT".to_owned(),
        ArrowDataType::UInt8 => "UTINYINT".to_owned(),
        ArrowDataType::UInt16 => "USMALLINT".to_owned(),
        ArrowDataType::UInt32 => "UINTEGER".to_owned(),
        ArrowDataType::UInt64 => "UBIGINT".to_owned(),
        ArrowDataType::Float32 => "FLOAT".to_owned(),
        ArrowDataType::Float64 => "DOUBLE".to_owned(),
        ArrowDataType::Decimal128(precision, scale) if *scale >= 0 => {
            format!("DECIMAL({precision}, {scale})")
        }
        ArrowDataType::Utf8 => "VARCHAR".to_owned(),
        ArrowDataType::Binary | ArrowDataType::FixedSizeBinary(_) => "BLOB".to_owned(),
        ArrowDataType::Date32 => "DATE".to_owned(),
        ArrowDataType::Time64(ArrowTimeUnit::Microsecond) => "TIME".to_owned(),
        ArrowDataType::Time64(ArrowTimeUnit::Nanosecond) => "TIME_NS".to_owned(),
        ArrowDataType::Timestamp(ArrowTimeUnit::Microsecond, None) => "TIMESTAMP".to_owned(),
        ArrowDataType::Timestamp(ArrowTimeUnit::Nanosecond, None) => "TIMESTAMP_NS".to_owned(),
        ArrowDataType::List(field) => format!("{}[]", arrow_type_sql(field.data_type())?),
        ArrowDataType::FixedSizeList(field, length) => {
            format!("{}[{length}]", arrow_type_sql(field.data_type())?)
        }
        ArrowDataType::Struct(fields) => {
            let fields = fields
                .iter()
                .map(|field| {
                    Ok(format!(
                        "{} {}",
                        quote_identifier(field.name()),
                        arrow_type_sql(field.data_type())?
                    ))
                })
                .collect::<Result<Vec<_>, DuckDbExecutionError>>()?;
            format!("STRUCT({})", fields.join(", "))
        }
        ArrowDataType::Map(field, _) => {
            let ArrowDataType::Struct(fields) = field.data_type() else {
                return Err(DuckDbExecutionError::UnsupportedArrowType {
                    data_type: data_type.to_string(),
                });
            };
            if fields.len() != 2 {
                return Err(DuckDbExecutionError::UnsupportedArrowType {
                    data_type: data_type.to_string(),
                });
            }
            format!(
                "MAP({}, {})",
                arrow_type_sql(fields[0].data_type())?,
                arrow_type_sql(fields[1].data_type())?
            )
        }
        unsupported => {
            return Err(DuckDbExecutionError::UnsupportedArrowType {
                data_type: unsupported.to_string(),
            });
        }
    };
    Ok(sql)
}

fn arrow_value(
    array: &dyn Array,
    index: usize,
    protocol_type: Option<&ProtocolDataType>,
) -> Result<Value, DuckDbExecutionError> {
    if array.is_null(index) {
        return Ok(Value::Null);
    }

    let protocol_type = protocol_type.map(strip_nullable);
    if matches!(protocol_type, Some(ProtocolDataType::Uuid)) {
        let array = downcast::<FixedSizeBinaryArray>(array)?;
        return Ok(Value::Text(uuid_text(array.value(index))?));
    }

    match array.data_type() {
        ArrowDataType::Boolean => Ok(Value::Boolean(
            downcast::<BooleanArray>(array)?.value(index),
        )),
        ArrowDataType::Int8 => Ok(Value::TinyInt(downcast::<Int8Array>(array)?.value(index))),
        ArrowDataType::Int16 => Ok(Value::SmallInt(downcast::<Int16Array>(array)?.value(index))),
        ArrowDataType::Int32 => Ok(Value::Int(downcast::<Int32Array>(array)?.value(index))),
        ArrowDataType::Int64 => Ok(Value::BigInt(downcast::<Int64Array>(array)?.value(index))),
        ArrowDataType::UInt8 => Ok(Value::UTinyInt(downcast::<UInt8Array>(array)?.value(index))),
        ArrowDataType::UInt16 => Ok(Value::USmallInt(
            downcast::<UInt16Array>(array)?.value(index),
        )),
        ArrowDataType::UInt32 => Ok(Value::UInt(downcast::<UInt32Array>(array)?.value(index))),
        ArrowDataType::UInt64 => Ok(Value::UBigInt(downcast::<UInt64Array>(array)?.value(index))),
        ArrowDataType::Float32 => Ok(Value::Float(downcast::<Float32Array>(array)?.value(index))),
        ArrowDataType::Float64 => Ok(Value::Double(downcast::<Float64Array>(array)?.value(index))),
        ArrowDataType::Decimal128(width, scale) if *scale >= 0 => {
            let scale =
                u8::try_from(*scale).map_err(|_| DuckDbExecutionError::InvalidArrowValue {
                    data_type: array.data_type().to_string(),
                })?;
            let decimal = Decimal::new(
                *width,
                scale,
                downcast::<Decimal128Array>(array)?.value(index),
            )
            .map_err(|error| DuckDbExecutionError::DuckDb {
                message: error.to_string(),
            })?;
            Ok(Value::Decimal(decimal))
        }
        ArrowDataType::Utf8 => Ok(Value::Text(
            downcast::<StringArray>(array)?.value(index).to_owned(),
        )),
        ArrowDataType::Binary => Ok(Value::Blob(
            downcast::<BinaryArray>(array)?.value(index).to_vec(),
        )),
        ArrowDataType::FixedSizeBinary(_) => Ok(Value::Blob(
            downcast::<FixedSizeBinaryArray>(array)?
                .value(index)
                .to_vec(),
        )),
        ArrowDataType::Date32 => Ok(Value::Date32(downcast::<Date32Array>(array)?.value(index))),
        ArrowDataType::Time64(ArrowTimeUnit::Microsecond) => Ok(Value::Time64(
            TimeUnit::Microsecond,
            downcast::<Time64MicrosecondArray>(array)?.value(index),
        )),
        ArrowDataType::Time64(ArrowTimeUnit::Nanosecond) => Ok(Value::Time64(
            TimeUnit::Nanosecond,
            downcast::<Time64NanosecondArray>(array)?.value(index),
        )),
        ArrowDataType::Timestamp(ArrowTimeUnit::Microsecond, None) => Ok(Value::Timestamp(
            TimeUnit::Microsecond,
            downcast::<TimestampMicrosecondArray>(array)?.value(index),
        )),
        ArrowDataType::Timestamp(ArrowTimeUnit::Nanosecond, None) => Ok(Value::Timestamp(
            TimeUnit::Nanosecond,
            downcast::<TimestampNanosecondArray>(array)?.value(index),
        )),
        ArrowDataType::List(_) => {
            let list = downcast::<ListArray>(array)?.value(index);
            let element_type = match protocol_type {
                Some(ProtocolDataType::Array { element, .. }) => element.as_deref(),
                _ => None,
            };
            Ok(Value::List(array_values(list.as_ref(), element_type)?))
        }
        ArrowDataType::FixedSizeList(_, _) => {
            let list = downcast::<FixedSizeListArray>(array)?.value(index);
            let element_type = match protocol_type {
                Some(ProtocolDataType::Array { element, .. }) => element.as_deref(),
                _ => None,
            };
            Ok(Value::Array(array_values(list.as_ref(), element_type)?))
        }
        ArrowDataType::Struct(_) => {
            let array = downcast::<StructArray>(array)?;
            let protocol_fields = match protocol_type {
                Some(ProtocolDataType::Struct { fields }) => Some(fields.as_slice()),
                _ => None,
            };
            let fields = match array.data_type() {
                ArrowDataType::Struct(fields) => fields,
                _ => {
                    return Err(DuckDbExecutionError::InvalidArrowValue {
                        data_type: array.data_type().to_string(),
                    });
                }
            };
            let mut values = Vec::with_capacity(fields.len());
            for (column_index, field) in fields.iter().enumerate() {
                let nested_type = protocol_fields
                    .and_then(|protocol| protocol.get(column_index))
                    .map(|field| field.data_type());
                values.push((
                    field.name().to_owned(),
                    arrow_value(array.column(column_index).as_ref(), index, nested_type)?,
                ));
            }
            Ok(Value::Struct(OrderedMap::from(values)))
        }
        ArrowDataType::Map(_, _) => {
            let map = downcast::<MapArray>(array)?.value(index);
            let (key_type, value_type) = match protocol_type {
                Some(ProtocolDataType::Map { key, value }) => {
                    (Some(key.as_ref()), Some(value.as_ref()))
                }
                _ => (None, None),
            };
            let mut entries = Vec::with_capacity(map.len());
            for entry_index in 0..map.len() {
                entries.push((
                    arrow_value(map.column(0).as_ref(), entry_index, key_type)?,
                    arrow_value(map.column(1).as_ref(), entry_index, value_type)?,
                ));
            }
            Ok(Value::Map(OrderedMap::from(entries)))
        }
        unsupported => Err(DuckDbExecutionError::UnsupportedArrowType {
            data_type: unsupported.to_string(),
        }),
    }
}

fn bind_expression(
    value: Value,
    parameters: &mut Vec<Value>,
) -> Result<String, DuckDbExecutionError> {
    match value {
        Value::List(values) | Value::Array(values) => {
            let expressions = values
                .into_iter()
                .map(|value| bind_expression(value, parameters))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", expressions.join(", ")))
        }
        Value::Struct(values) => {
            let expressions = values
                .iter()
                .map(|(name, value)| {
                    Ok(format!(
                        "{}: {}",
                        quote_literal(name),
                        bind_expression(value.clone(), parameters)?
                    ))
                })
                .collect::<Result<Vec<_>, DuckDbExecutionError>>()?;
            Ok(format!("{{{}}}", expressions.join(", ")))
        }
        Value::Map(values) => {
            let mut keys = Vec::new();
            let mut mapped_values = Vec::new();
            for (key, value) in values.iter() {
                keys.push(bind_expression(key.clone(), parameters)?);
                mapped_values.push(bind_expression(value.clone(), parameters)?);
            }
            Ok(format!(
                "MAP([{}], [{}])",
                keys.join(", "),
                mapped_values.join(", ")
            ))
        }
        Value::Enum(value) => {
            parameters.push(Value::Text(value));
            Ok("?".to_owned())
        }
        Value::Union(_) => Err(DuckDbExecutionError::InvalidArrowValue {
            data_type: "DuckDB UNION parameter materialization is unsupported".to_owned(),
        }),
        scalar => {
            parameters.push(scalar);
            Ok("?".to_owned())
        }
    }
}

fn strip_nullable(data_type: &ProtocolDataType) -> &ProtocolDataType {
    match data_type {
        ProtocolDataType::Nullable(inner) => strip_nullable(inner),
        other => other,
    }
}

fn array_values(
    array: &dyn Array,
    protocol_type: Option<&ProtocolDataType>,
) -> Result<Vec<Value>, DuckDbExecutionError> {
    (0..array.len())
        .map(|index| arrow_value(array, index, protocol_type))
        .collect()
}

fn downcast<T: Array + 'static>(array: &dyn Array) -> Result<&T, DuckDbExecutionError> {
    array
        .as_any()
        .downcast_ref::<T>()
        .ok_or_else(|| DuckDbExecutionError::InvalidArrowValue {
            data_type: array.data_type().to_string(),
        })
}

fn uuid_text(bytes: &[u8]) -> Result<String, DuckDbExecutionError> {
    let bytes: &[u8; 16] =
        bytes
            .try_into()
            .map_err(|_| DuckDbExecutionError::InvalidArrowValue {
                data_type: format!("UUID requires 16 bytes, received {}", bytes.len()),
            })?;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    ))
}

fn canonical_value(value: &Value) -> Result<String, DuckDbExecutionError> {
    let encoded = match value {
        Value::Null => "null".to_owned(),
        Value::Boolean(value) => format!("bool:{value}"),
        Value::TinyInt(value) => format!("i8:{value}"),
        Value::SmallInt(value) => format!("i16:{value}"),
        Value::Int(value) => format!("i32:{value}"),
        Value::BigInt(value) => format!("i64:{value}"),
        Value::HugeInt(value) => format!("i128:{value}"),
        Value::UHugeInt(value) => format!("u128:{value}"),
        Value::UTinyInt(value) => format!("u8:{value}"),
        Value::USmallInt(value) => format!("u16:{value}"),
        Value::UInt(value) => format!("u32:{value}"),
        Value::UBigInt(value) => format!("u64:{value}"),
        Value::Float(value) => format!("f32:{:08x}", value.to_bits()),
        Value::Double(value) => format!("f64:{:016x}", value.to_bits()),
        Value::Decimal(value) => format!(
            "decimal:{}:{}:{}",
            value.width(),
            value.scale(),
            value.value()
        ),
        Value::Timestamp(unit, value) => format!("timestamp:{}:{value}", time_unit_name(*unit)),
        Value::Text(value) => format!("text:{}", frame(value)),
        Value::Blob(value) => format!("blob:{}", hex(value)),
        Value::Geometry(value) => format!("geometry:{}", hex(value)),
        Value::Date32(value) => format!("date32:{value}"),
        Value::Time64(unit, value) => format!("time64:{}:{value}", time_unit_name(*unit)),
        Value::Interval {
            months,
            days,
            nanos,
        } => format!("interval:{months}:{days}:{nanos}"),
        Value::List(values) => format!("list:{}", canonical_values(values)?),
        Value::Enum(value) => format!("enum:{}", frame(value)),
        Value::Struct(values) => {
            let entries = values
                .iter()
                .map(|(key, value)| {
                    Ok(format!("{}{}", frame(key), frame(&canonical_value(value)?)))
                })
                .collect::<Result<Vec<_>, DuckDbExecutionError>>()?;
            format!("struct:{}", frame(&entries.join("")))
        }
        Value::Array(values) => format!("array:{}", canonical_values(values)?),
        Value::Map(values) => {
            let entries = values
                .iter()
                .map(|(key, value)| {
                    Ok(format!(
                        "{}{}",
                        frame(&canonical_value(key)?),
                        frame(&canonical_value(value)?)
                    ))
                })
                .collect::<Result<Vec<_>, DuckDbExecutionError>>()?;
            format!("map:{}", frame(&entries.join("")))
        }
        Value::Union(value) => format!("union:{}", frame(&canonical_value(value)?)),
        unsupported => {
            return Err(DuckDbExecutionError::DuckDb {
                message: format!("unsupported DuckDB result value {unsupported:?}"),
            });
        }
    };
    Ok(encoded)
}

fn canonical_values(values: &[Value]) -> Result<String, DuckDbExecutionError> {
    let encoded = values
        .iter()
        .map(canonical_value)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|value| frame(&value))
        .collect::<String>();
    Ok(frame(&encoded))
}

const fn time_unit_name(unit: TimeUnit) -> &'static str {
    match unit {
        TimeUnit::Second => "s",
        TimeUnit::Millisecond => "ms",
        TimeUnit::Microsecond => "us",
        TimeUnit::Nanosecond => "ns",
    }
}

fn frame(value: &str) -> String {
    format!("{}:{value}", value.len())
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}
