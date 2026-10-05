use sql_semantic_protocol::{DataType, DataTypeField, parse_data_type};
use sql_tdg::{
    ApprovedResult, GeneratedRelation, GenerationBoundary, GenerationRowCounts, ProtocolSnapshot,
    RelationSchema, SchemaColumn, TestCaseMetadata, TestTarget, WorkloadIdentity,
    generate_classified_from_sql, generate_from_sql,
};
pub use sql_tdg::{
    GeneratedData, QueryResult, ResultColumn, ResultOrdering, Table, TableError, TestCase,
    TestCaseError, VerificationError,
};

#[path = "support/duckdb.rs"]
mod duckdb;

use duckdb::DuckDbExecutor;

fn column(name: &str, data_type: DataType) -> SchemaColumn {
    SchemaColumn::new(name, data_type).expect("test schema column should be valid")
}

fn typed_schema() -> RelationSchema {
    RelationSchema::new(
        "typed_source",
        vec![
            column("i64_value", DataType::SignedInteger { bits: Some(64) }),
            column("u64_value", DataType::UnsignedInteger { bits: Some(64) }),
            column(
                "decimal_value",
                DataType::Decimal {
                    precision: Some(18),
                    scale: Some(4),
                },
            ),
            column("float_value", DataType::FloatingPoint { bits: Some(64) }),
            column(
                "text_value",
                DataType::String {
                    length: Some(32),
                    fixed: false,
                },
            ),
            column(
                "binary_value",
                DataType::Binary {
                    length: Some(16),
                    fixed: false,
                },
            ),
            column("date_value", DataType::Date),
            column("time_value", DataType::Time { precision: Some(6) }),
            column(
                "timestamp_value",
                DataType::Timestamp { precision: Some(9) },
            ),
            column("uuid_value", DataType::Uuid),
            column("json_value", DataType::Json),
            column(
                "nullable_value",
                DataType::Nullable(Box::new(DataType::SignedInteger { bits: Some(64) })),
            ),
            column(
                "array_value",
                DataType::Array {
                    element: Some(Box::new(DataType::SignedInteger { bits: Some(64) })),
                    length: None,
                },
            ),
            column(
                "struct_value",
                DataType::Struct {
                    fields: vec![
                        DataTypeField::new(
                            Some("id".to_owned()),
                            DataType::SignedInteger { bits: Some(64) },
                        ),
                        DataTypeField::new(
                            Some("label".to_owned()),
                            DataType::String {
                                length: None,
                                fixed: false,
                            },
                        ),
                    ],
                },
            ),
            column(
                "map_value",
                DataType::Map {
                    key: Box::new(DataType::String {
                        length: None,
                        fixed: false,
                    }),
                    value: Box::new(DataType::SignedInteger { bits: Some(64) }),
                },
            ),
            column(
                "enum_value",
                parse_data_type("ENUM('ready', 'done')", "mysql")
                    .expect("enum datatype should normalize"),
            ),
            column(
                "set_value",
                parse_data_type("SET('red', 'blue')", "mysql")
                    .expect("set datatype should normalize"),
            ),
        ],
    )
    .expect("test relation schema should be valid")
}

fn test_case() -> TestCase {
    let metadata = TestCaseMetadata::new(
        WorkloadIdentity::raw_sql("orders regression", "queries/orders.sql")
            .expect("workload should be valid"),
        TestTarget::relation("analytics.orders").expect("target should be valid"),
        GenerationBoundary::physical_sources(),
        "duckdb",
        42,
        ProtocolSnapshot::new("{\"protocol_version\":\"1.0\"}")
            .expect("protocol snapshot should be valid"),
        vec![GeneratedRelation::new("orders", 4, 2).expect("relation should be valid")],
    )
    .expect("metadata should be valid");
    TestCase::new(metadata)
}

#[test]
fn materializes_every_task_12_type_shared_with_duckdb() {
    let generated = generate_from_sql(
        "SELECT * FROM typed_source",
        "duckdb",
        &[typed_schema()],
        8,
        42,
    )
    .expect("typed generation should succeed");
    let executor = DuckDbExecutor::in_memory().expect("DuckDB should open");
    executor
        .materialize(&generated)
        .expect("generated data should materialize");

    let result = executor
        .execute("SELECT * FROM typed_source ORDER BY i64_value")
        .expect("materialized relation should query");
    assert_eq!(result.rows().len(), 8);

    let types = executor
        .execute(
            "SELECT typeof(i64_value), typeof(u64_value), typeof(decimal_value), \
                    typeof(float_value), typeof(text_value), typeof(binary_value), \
                    typeof(date_value), typeof(time_value), typeof(timestamp_value), \
                    typeof(uuid_value), typeof(json_value), typeof(nullable_value), \
                    typeof(array_value), typeof(struct_value), typeof(map_value), \
                    typeof(enum_value), typeof(set_value) \
             FROM typed_source LIMIT 1",
        )
        .expect("logical types should query");
    let row = &types.rows()[0];

    for expected in [
        "BIGINT",
        "UBIGINT",
        "DECIMAL(18,4)",
        "DOUBLE",
        "VARCHAR",
        "BLOB",
        "DATE",
        "TIME",
        "TIMESTAMP_NS",
        "UUID",
        "JSON",
        "BIGINT",
        "BIGINT[]",
        "STRUCT",
        "MAP",
        "ENUM",
        "VARCHAR",
    ] {
        assert!(
            row.iter().any(|value| value.contains(expected)),
            "expected a DuckDB logical type containing {expected:?}, got {row:?}"
        );
    }

    let semantic_values = executor
        .execute(
            "SELECT i64_value, u64_value, decimal_value::VARCHAR, float_value, text_value, \
                    hex(binary_value), date_value::VARCHAR, time_value::VARCHAR, \
                    timestamp_value::VARCHAR, uuid_value::VARCHAR, json_value::VARCHAR, \
                    nullable_value, array_value::VARCHAR, struct_value::VARCHAR, \
                    map_value::VARCHAR, enum_value::VARCHAR, set_value \
             FROM typed_source ORDER BY i64_value",
        )
        .expect("all materialized values should be readable");
    assert_eq!(semantic_values.rows().len(), 8);
    assert!(
        semantic_values
            .rows()
            .iter()
            .all(|row| row.len() == typed_schema().columns().len())
    );
}

#[test]
fn qualified_relations_and_multi_statement_workloads_execute_in_order() {
    let schema = RelationSchema::new(
        "raw.orders",
        vec![column("amount", DataType::SignedInteger { bits: Some(32) })],
    )
    .expect("schema should be valid");
    let generated = generate_from_sql(
        "SELECT amount FROM raw.orders WHERE amount >= 10",
        "duckdb",
        &[schema],
        4,
        7,
    )
    .expect("generation should succeed");
    let executor = DuckDbExecutor::in_memory().expect("DuckDB should open");
    executor
        .materialize(&generated)
        .expect("data should materialize");

    let result = executor
        .execute(
            "CREATE TEMP TABLE copied AS SELECT * FROM raw.orders;\
             UPDATE copied SET amount = amount + 1;\
             SELECT amount FROM copied ORDER BY amount",
        )
        .expect("ordered workload should execute");
    assert_eq!(result.rows().len(), 4);
}

#[test]
fn approval_snapshot_round_trips_and_filter_regressions_fail() {
    let schema = RelationSchema::new(
        "orders",
        vec![column("amount", DataType::SignedInteger { bits: Some(32) })],
    )
    .expect("schema should be valid");
    let generated = generate_classified_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
        "duckdb",
        &[schema],
        GenerationRowCounts::new(4, 2).expect("classified row counts should be valid"),
        42,
    )
    .expect("classified generation should succeed");
    let executor = DuckDbExecutor::in_memory().expect("DuckDB should open");
    executor
        .materialize(&generated)
        .expect("data should materialize");

    let mut test_case = test_case();
    executor
        .approve_workload(
            &mut test_case,
            "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20 ORDER BY amount",
            ResultOrdering::Ordered,
        )
        .expect("baseline should approve");

    let serialized = test_case
        .approved_result()
        .expect("approval should exist")
        .serialize();
    let decoded =
        ApprovedResult::deserialize(&serialized).expect("approval snapshot should round-trip");
    assert_eq!(
        &decoded,
        test_case
            .approved_result()
            .expect("approval should remain present")
    );

    executor
        .verify_workload(
            &test_case,
            "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20 ORDER BY amount",
        )
        .expect("unchanged filter should verify");

    assert!(
        executor
            .verify_workload(&test_case, "SELECT amount FROM orders ORDER BY amount")
            .is_err(),
        "admitting rejected rows must fail verification"
    );
    assert!(
        executor
            .verify_workload(
                &test_case,
                "SELECT amount FROM orders \
                 WHERE amount >= 10 AND amount < 20 AND 1 = 0 ORDER BY amount",
            )
            .is_err(),
        "a stricter filter that loses approved rows must fail verification"
    );
}

#[test]
fn deterministic_generation_reproduces_the_same_duckdb_result() {
    let schema = RelationSchema::new(
        "orders",
        vec![column("amount", DataType::SignedInteger { bits: Some(32) })],
    )
    .expect("schema should be valid");
    let first = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount <= 20",
        "duckdb",
        std::slice::from_ref(&schema),
        16,
        99,
    )
    .expect("first generation should succeed");
    let second = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount <= 20",
        "duckdb",
        &[schema],
        16,
        99,
    )
    .expect("second generation should succeed");

    let first_db = DuckDbExecutor::in_memory().expect("first DuckDB should open");
    let second_db = DuckDbExecutor::in_memory().expect("second DuckDB should open");
    first_db
        .materialize(&first)
        .expect("first data should materialize");
    second_db
        .materialize(&second)
        .expect("second data should materialize");

    let query = "SELECT amount FROM orders ORDER BY amount";
    assert_eq!(
        first_db.execute(query).expect("first query should run"),
        second_db.execute(query).expect("second query should run")
    );
}

#[test]
fn read_only_database_does_not_allow_fixture_mutation() {
    let schema = RelationSchema::new(
        "orders",
        vec![column("amount", DataType::SignedInteger { bits: Some(32) })],
    )
    .expect("schema should be valid");
    let generated = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10",
        "duckdb",
        &[schema],
        4,
        13,
    )
    .expect("generation should succeed");

    let path = std::env::temp_dir().join(format!(
        "sql-tdg-duckdb-execution-{}.duckdb",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);
    {
        let writer = DuckDbExecutor::create(&path).expect("writable database should open");
        writer
            .materialize(&generated)
            .expect("explicit database output should materialize");
    }
    {
        let reader = DuckDbExecutor::open_read_only(&path).expect("read-only database should open");
        assert_eq!(
            reader
                .execute("SELECT amount FROM orders")
                .expect("read-only query should execute")
                .rows()
                .len(),
            4
        );
        assert!(
            reader.execute("DELETE FROM orders").is_err(),
            "read-only verification must reject fixture writes"
        );
    }
    std::fs::remove_file(path).expect("test database should be removable");
}
