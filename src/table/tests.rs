use std::collections::BTreeMap;

use arrow_array::{
    Array, BooleanArray, Int32Array, StringArray, TimestampMicrosecondArray,
};
use arrow_schema::{DataType, TimeUnit};
use chrono::{Duration, TimeZone, Utc};

use crate::{Column, ColumnType};

use super::{Table, TableError, TableValue};

fn column(name: &str, column_type: ColumnType) -> Column {
    Column::new(name, column_type)
}

fn full_schema() -> Vec<Column> {
    vec![
        column("int_col", ColumnType::Int),
        column("timestamp_col", ColumnType::Timestamp),
        column("bool_col", ColumnType::Bool),
        column("string_col", ColumnType::String),
    ]
}

#[test]
fn append() {
    let mut table =
        Table::new(vec![column("col_a", ColumnType::Int)], 1).expect("valid table schema");

    table
        .append("col_a", TableValue::Int(10))
        .expect("integer should append");

    assert_eq!(table.dim().rows(), 1);
    assert_eq!(table.dim().cols(), 1);
}

#[test]
fn append_rejects_wrong_type() {
    let mut table =
        Table::new(vec![column("col_a", ColumnType::Int)], 1).expect("valid table schema");

    let error = table
        .append("col_a", TableValue::String("10".to_owned()))
        .expect_err("wrong value type should be rejected");

    assert_eq!(
        error,
        TableError::ColumnTypeMismatch {
            column: "col_a".to_owned(),
            expected: ColumnType::Int,
            actual: ColumnType::String,
        }
    );
}

#[test]
fn append_rejects_unknown_column() {
    let mut table =
        Table::new(vec![column("col_a", ColumnType::Int)], 1).expect("valid table schema");

    let error = table
        .append("missing", TableValue::Int(10))
        .expect_err("unknown column should be rejected");

    assert_eq!(
        error,
        TableError::UnknownColumn {
            column: "missing".to_owned(),
        }
    );
}

#[test]
fn wipe() {
    let timestamp = Utc
        .timestamp_opt(10, 0)
        .single()
        .expect("valid UTC timestamp");
    let mut table = Table::new(full_schema(), 1).expect("valid table schema");

    table
        .append("int_col", TableValue::Int(10))
        .expect("integer should append");
    table
        .append("timestamp_col", TableValue::Timestamp(timestamp))
        .expect("timestamp should append");
    table
        .append("bool_col", TableValue::Bool(true))
        .expect("boolean should append");
    table
        .append("string_col", TableValue::String("value".to_owned()))
        .expect("string should append");
    table.build_all();

    table.wipe();

    assert_eq!(
        table.get_all_ints().expect("integer getter should work"),
        BTreeMap::from([("int_col".to_owned(), None)])
    );
    assert_eq!(
        table
            .get_all_timestamps()
            .expect("timestamp getter should work"),
        BTreeMap::from([("timestamp_col".to_owned(), None)])
    );
    assert_eq!(
        table.get_all_bools().expect("boolean getter should work"),
        BTreeMap::from([("bool_col".to_owned(), None)])
    );
    assert_eq!(
        table.get_all_strings().expect("string getter should work"),
        BTreeMap::from([("string_col".to_owned(), None)])
    );

    table
        .append("int_col", TableValue::Int(20))
        .expect("wiped builder should be reusable");
    table.build_ints();
    assert_eq!(
        table.get_ints("int_col").expect("integer getter should work"),
        Some(vec![20])
    );
}

#[test]
fn sort_ints() {
    let cases = [
        (
            vec![column("col1", ColumnType::Int)],
            BTreeMap::from([("col1", vec![7, 2, 6, 3, 1])]),
            BTreeMap::from([("col1".to_owned(), Some(vec![1, 2, 3, 6, 7]))]),
        ),
        (
            vec![
                column("col1", ColumnType::Int),
                column("col2", ColumnType::Int),
            ],
            BTreeMap::from([
                ("col1", vec![5, 3, 9]),
                ("col2", vec![2, 2, 1]),
            ]),
            BTreeMap::from([
                ("col1".to_owned(), Some(vec![3, 5, 9])),
                ("col2".to_owned(), Some(vec![1, 2, 2])),
            ]),
        ),
    ];

    for (schema, input, expected) in cases {
        let row_count = input.values().next().map_or(0, Vec::len);
        let mut table = Table::new(schema, row_count).expect("valid table schema");

        for (column, values) in input {
            for value in values {
                table
                    .append(column, TableValue::Int(value))
                    .expect("integer should append");
            }
        }

        table.build_ints();
        table.sort_ints().expect("integer sort should succeed");

        assert_eq!(
            table.get_all_ints().expect("integer getter should work"),
            expected
        );
    }
}

#[test]
fn sort_timestamps() {
    let base = Utc
        .with_ymd_and_hms(2026, 10, 1, 9, 30, 0)
        .single()
        .expect("valid UTC timestamp");
    let mut table =
        Table::new(vec![column("ts", ColumnType::Timestamp)], 3).expect("valid table schema");

    for timestamp in [base + Duration::seconds(2), base, base + Duration::seconds(1)] {
        table
            .append("ts", TableValue::Timestamp(timestamp))
            .expect("timestamp should append");
    }

    table.build_timestamps();
    table
        .sort_timestamps()
        .expect("timestamp sort should succeed");

    assert_eq!(
        table
            .get_timestamps("ts")
            .expect("timestamp getter should work"),
        Some(vec![
            base,
            base + Duration::seconds(1),
            base + Duration::seconds(2),
        ])
    );
}

#[test]
fn all_column_types_use_arrow_storage() {
    let timestamp = Utc
        .with_ymd_and_hms(2026, 10, 1, 9, 30, 0)
        .single()
        .expect("valid UTC timestamp");
    let mut table = Table::new(full_schema(), 2).expect("valid table schema");

    for value in [7, 3] {
        table
            .append("int_col", TableValue::Int(value))
            .expect("integer should append");
    }
    for value in [timestamp, timestamp + Duration::seconds(1)] {
        table
            .append("timestamp_col", TableValue::Timestamp(value))
            .expect("timestamp should append");
    }
    for value in [true, false] {
        table
            .append("bool_col", TableValue::Bool(value))
            .expect("boolean should append");
    }
    for value in ["alpha", "beta"] {
        table
            .append("string_col", TableValue::String(value.to_owned()))
            .expect("string should append");
    }

    table.build_all();

    assert_eq!(
        table.get_all_ints().expect("integer getter should work"),
        BTreeMap::from([("int_col".to_owned(), Some(vec![7, 3]))])
    );
    assert_eq!(
        table
            .get_all_timestamps()
            .expect("timestamp getter should work"),
        BTreeMap::from([(
            "timestamp_col".to_owned(),
            Some(vec![timestamp, timestamp + Duration::seconds(1)]),
        )])
    );
    assert_eq!(
        table.get_all_bools().expect("boolean getter should work"),
        BTreeMap::from([("bool_col".to_owned(), Some(vec![true, false]))])
    );
    assert_eq!(
        table.get_all_strings().expect("string getter should work"),
        BTreeMap::from([(
            "string_col".to_owned(),
            Some(vec!["alpha".to_owned(), "beta".to_owned()]),
        )])
    );

    let expected_types = [
        ("int_col", DataType::Int32),
        (
            "timestamp_col",
            DataType::Timestamp(TimeUnit::Microsecond, None),
        ),
        ("bool_col", DataType::Boolean),
        ("string_col", DataType::Utf8),
    ];
    for (column, expected_type) in expected_types {
        let storage = table
            .columns
            .get(column)
            .expect("schema column should have storage");
        let array = storage.array.as_ref().expect("column should be built");
        assert_eq!(array.data_type(), &expected_type);
    }

    assert!(
        table.columns["int_col"]
            .array
            .as_ref()
            .expect("integer column should be built")
            .as_any()
            .downcast_ref::<Int32Array>()
            .is_some()
    );
    assert!(
        table.columns["timestamp_col"]
            .array
            .as_ref()
            .expect("timestamp column should be built")
            .as_any()
            .downcast_ref::<TimestampMicrosecondArray>()
            .is_some()
    );
    assert!(
        table.columns["bool_col"]
            .array
            .as_ref()
            .expect("boolean column should be built")
            .as_any()
            .downcast_ref::<BooleanArray>()
            .is_some()
    );
    assert!(
        table.columns["string_col"]
            .array
            .as_ref()
            .expect("string column should be built")
            .as_any()
            .downcast_ref::<StringArray>()
            .is_some()
    );
}

#[test]
fn build_behavior_is_independent_per_type() {
    let timestamp = Utc
        .timestamp_opt(10, 0)
        .single()
        .expect("valid UTC timestamp");
    let mut table = Table::new(full_schema(), 1).expect("valid table schema");

    table
        .append("int_col", TableValue::Int(1))
        .expect("integer should append");
    table
        .append("timestamp_col", TableValue::Timestamp(timestamp))
        .expect("timestamp should append");
    table
        .append("bool_col", TableValue::Bool(true))
        .expect("boolean should append");
    table
        .append("string_col", TableValue::String("value".to_owned()))
        .expect("string should append");

    table.build_ints();

    assert_eq!(
        table.get_ints("int_col").expect("integer getter should work"),
        Some(vec![1])
    );
    assert_eq!(
        table
            .get_timestamps("timestamp_col")
            .expect("timestamp getter should work"),
        None
    );
    assert_eq!(
        table
            .get_bools("bool_col")
            .expect("boolean getter should work"),
        None
    );
    assert_eq!(
        table
            .get_strings("string_col")
            .expect("string getter should work"),
        None
    );

    table.build_timestamps();
    table.build_bools();
    table.build_strings();

    assert_eq!(
        table
            .get_timestamps("timestamp_col")
            .expect("timestamp getter should work"),
        Some(vec![timestamp])
    );
    assert_eq!(
        table
            .get_bools("bool_col")
            .expect("boolean getter should work"),
        Some(vec![true])
    );
    assert_eq!(
        table
            .get_strings("string_col")
            .expect("string getter should work"),
        Some(vec!["value".to_owned()])
    );
}
