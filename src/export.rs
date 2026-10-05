//! Backend-neutral Arrow and file export helpers.

use std::error::Error;
use std::fmt;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use arrow_array::{RecordBatch, make_array};
use arrow_csv::WriterBuilder;
use arrow_schema::ArrowError;
use parquet::arrow::ArrowWriter;
use parquet::errors::ParquetError;

use crate::{Table, TableError};

/// Errors returned while exposing generated data as Arrow or writing file outputs.
#[derive(Debug)]
pub enum ExportError {
    /// A table column has not been finalized into an Arrow array.
    UnbuiltColumn {
        /// Column that has no finalized array.
        column: String,
    },
    /// A table lookup or invariant failed.
    Table(TableError),
    /// Arrow rejected a record batch or CSV write.
    Arrow(ArrowError),
    /// Parquet rejected the Arrow schema or values.
    Parquet(ParquetError),
    /// The output file could not be created or written.
    Io(std::io::Error),
}

impl fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnbuiltColumn { column } => {
                write!(formatter, "column {column:?} has no finalized Arrow array")
            }
            Self::Table(error) => error.fmt(formatter),
            Self::Arrow(error) => write!(formatter, "Arrow export failed: {error}"),
            Self::Parquet(error) => write!(formatter, "Parquet export failed: {error}"),
            Self::Io(error) => write!(formatter, "output file operation failed: {error}"),
        }
    }
}

impl Error for ExportError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnbuiltColumn { .. } => None,
            Self::Table(error) => Some(error),
            Self::Arrow(error) => Some(error),
            Self::Parquet(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<TableError> for ExportError {
    fn from(error: TableError) -> Self {
        Self::Table(error)
    }
}

impl From<ArrowError> for ExportError {
    fn from(error: ArrowError) -> Self {
        Self::Arrow(error)
    }
}

impl From<ParquetError> for ExportError {
    fn from(error: ParquetError) -> Self {
        Self::Parquet(error)
    }
}

impl From<std::io::Error> for ExportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

/// Exposes one finalized generated table as an Arrow record batch.
pub fn record_batch(table: &Table) -> Result<RecordBatch, ExportError> {
    let mut columns = Vec::with_capacity(table.arrow_schema().fields().len());
    for field in table.arrow_schema().fields() {
        let array = table
            .array(field.name())?
            .ok_or_else(|| ExportError::UnbuiltColumn {
                column: field.name().to_owned(),
            })?;
        columns.push(make_array(array.to_data()));
    }

    RecordBatch::try_new(Arc::new(table.arrow_schema().clone()), columns).map_err(Into::into)
}

/// Writes one finalized table as CSV with a header row.
///
/// CSV is an interoperability format and inherits Arrow CSV's type limitations. Use Parquet when
/// nested or otherwise non-tabular Arrow types must be preserved losslessly.
pub fn write_csv(table: &Table, path: impl AsRef<Path>) -> Result<(), ExportError> {
    let batch = record_batch(table)?;
    let file = File::create(path)?;
    let mut writer = WriterBuilder::new().with_header(true).build(file);
    writer.write(&batch)?;
    Ok(())
}

/// Writes one finalized table as Parquet while preserving its Arrow schema metadata.
pub fn write_parquet(table: &Table, path: impl AsRef<Path>) -> Result<(), ExportError> {
    let batch = record_batch(table)?;
    let file = File::create(path)?;
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None)?;
    writer.write(&batch)?;
    writer.close()?;
    Ok(())
}
