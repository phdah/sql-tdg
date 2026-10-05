use std::fs::File;

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use sql_semantic_protocol::DataType;
use sql_tdg::{
    RelationSchema, SchemaColumn, generate_from_sql, record_batch, write_csv, write_parquet,
};

fn column(name: &str, data_type: DataType) -> SchemaColumn {
    SchemaColumn::new(name, data_type).expect("test schema column should be valid")
}

fn output_path(extension: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "sql-tdg-export-{}-{}.{}",
        std::process::id(),
        extension,
        extension
    ))
}

#[test]
fn exposes_generated_table_as_record_batch_and_csv() {
    let schema = RelationSchema::new(
        "orders",
        vec![
            column("amount", DataType::SignedInteger { bits: Some(32) }),
            column(
                "label",
                DataType::String {
                    length: Some(32),
                    fixed: false,
                },
            ),
        ],
    )
    .expect("schema should be valid");
    let generated = generate_from_sql(
        "SELECT amount, label FROM orders WHERE amount >= 10",
        "duckdb",
        &[schema],
        4,
        42,
    )
    .expect("generation should succeed");
    let table = generated
        .table("orders")
        .expect("orders should be generated");

    let batch = record_batch(table).expect("record batch should materialize");
    assert_eq!(batch.num_rows(), 4);
    assert_eq!(batch.num_columns(), 2);

    let path = output_path("csv");
    let _ = std::fs::remove_file(&path);
    write_csv(table, &path).expect("CSV export should succeed");
    let contents = std::fs::read_to_string(&path).expect("CSV should be readable");
    assert!(contents.starts_with("amount,label\n"));
    assert_eq!(contents.lines().count(), 5);
    std::fs::remove_file(path).expect("CSV should be removable");
}

#[test]
fn parquet_round_trip_preserves_arrow_schema_and_rows() {
    let schema = RelationSchema::new(
        "events",
        vec![
            column("id", DataType::SignedInteger { bits: Some(64) }),
            column(
                "tags",
                DataType::Array {
                    element: Some(Box::new(DataType::SignedInteger { bits: Some(64) })),
                    length: None,
                },
            ),
            column("payload", DataType::Json),
        ],
    )
    .expect("schema should be valid");
    let generated = generate_from_sql(
        "SELECT id, tags, payload FROM events",
        "duckdb",
        &[schema],
        5,
        7,
    )
    .expect("generation should succeed");
    let table = generated
        .table("events")
        .expect("events should be generated");
    let expected = record_batch(table).expect("record batch should materialize");

    let path = output_path("parquet");
    let _ = std::fs::remove_file(&path);
    write_parquet(table, &path).expect("Parquet export should succeed");

    let reader =
        ParquetRecordBatchReaderBuilder::try_new(File::open(&path).expect("Parquet should open"))
            .expect("Parquet reader should initialize")
            .build()
            .expect("Parquet reader should build");
    let batches = reader
        .collect::<Result<Vec<_>, _>>()
        .expect("Parquet should read back");

    assert_eq!(batches.len(), 1);
    assert_eq!(batches[0].num_rows(), expected.num_rows());
    assert_eq!(batches[0].schema(), expected.schema());
    std::fs::remove_file(path).expect("Parquet should be removable");
}
