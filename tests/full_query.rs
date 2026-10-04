use sql_tdg::{
    DataType, ProtocolGenerationError, RelationSchema, SchemaColumn, generate_from_sql,
    to_timestamp,
};

const ROWS: usize = 12;
const SEED: u64 = 42;

fn schema(relation: &str, columns: &[(&str, &str)]) -> RelationSchema {
    RelationSchema::new(
        relation,
        columns
            .iter()
            .map(|(name, data_type)| {
                SchemaColumn::from_sql_type(*name, data_type, "generic")
                    .expect("test datatype should normalize")
            })
            .collect(),
    )
    .expect("test relation schema should be valid")
}

#[test]
fn protocol_driven_integer_equality_generation() {
    let generated = generate_from_sql(
        "SELECT col_a FROM t WHERE col_a = 10",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect("protocol-driven generation should succeed");

    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert_eq!(values, vec![10; ROWS]);
}

#[test]
fn protocol_ranges_preserve_inclusive_exclusive_bounds_and_exclusions() {
    let sql = "SELECT col_a FROM t WHERE col_a >= 4 AND col_a < 8 AND col_a != 6";
    let schemas = [schema("t", &[("col_a", "INTEGER")])];

    let first =
        generate_from_sql(sql, "generic", &schemas, ROWS, SEED).expect("generation should succeed");
    let second =
        generate_from_sql(sql, "generic", &schemas, ROWS, SEED).expect("generation should repeat");

    let first_values = first
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");
    let second_values = second
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert_eq!(first_values, second_values);
    assert!(
        first_values
            .iter()
            .all(|value| (4..8).contains(value) && *value != 6)
    );
}

#[test]
fn protocol_driven_boolean_generation() {
    let generated = generate_from_sql(
        "SELECT enabled FROM t WHERE enabled = true",
        "generic",
        &[schema("t", &[("enabled", "BOOLEAN")])],
        ROWS,
        SEED,
    )
    .expect("protocol-driven generation should succeed");

    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_bools("enabled")
        .expect("boolean column should be readable")
        .expect("boolean column should be built");

    assert_eq!(values, vec![true; ROWS]);
}

#[test]
fn protocol_driven_timestamp_generation() {
    let literal = "2013-06-17T14:29:00Z";
    let generated = generate_from_sql(
        &format!("SELECT created_at FROM t WHERE created_at = TIMESTAMP '{literal}'"),
        "generic",
        &[schema("t", &[("created_at", "TIMESTAMP")])],
        ROWS,
        SEED,
    )
    .expect("protocol-driven generation should succeed");

    let expected = i64::from(to_timestamp(literal).expect("test timestamp should be valid"));
    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_timestamps("created_at")
        .expect("timestamp column should be readable")
        .expect("timestamp column should be built");

    assert_eq!(values.len(), ROWS);
    assert!(values.iter().all(|value| value.timestamp() == expected));
}

#[test]
fn terminal_composed_semantics_drive_source_generation() {
    let sql = "
        CREATE VIEW stage_orders AS
        SELECT amount FROM raw_orders WHERE amount >= 5;
        CREATE VIEW final_orders AS
        SELECT amount FROM stage_orders WHERE amount <= 10;
    ";
    let generated = generate_from_sql(
        sql,
        "generic",
        &[schema("raw_orders", &[("amount", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect("transitive protocol semantics should generate source data");

    let values = generated
        .table("raw_orders")
        .expect("physical source table should exist")
        .get_ints("amount")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert!(values.iter().all(|value| (5..=10).contains(value)));
}

#[test]
fn contradictory_protocol_domain_is_an_explicit_error() {
    let error = generate_from_sql(
        "SELECT col_a FROM t WHERE col_a > 10 AND col_a < 5",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("contradictory domain must not generate rows");

    assert_eq!(
        error,
        ProtocolGenerationError::EmptyDomain {
            relation: "t".to_owned(),
            column: "col_a".to_owned(),
        }
    );
}

#[test]
fn multiple_terminal_outcomes_require_selection() {
    let error = generate_from_sql(
        "SELECT col_a FROM t; SELECT col_a FROM t",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("multiple outcomes must not be selected arbitrarily");

    assert!(matches!(
        error,
        ProtocolGenerationError::AmbiguousTerminalOutcome { .. }
    ));
}

#[test]
fn missing_source_schema_is_an_explicit_error() {
    let error = generate_from_sql(
        "SELECT col_a FROM t WHERE col_a = 10",
        "generic",
        &[],
        ROWS,
        SEED,
    )
    .expect_err("missing source type information must fail");

    assert_eq!(
        error,
        ProtocolGenerationError::MissingSourceSchema {
            relation: "t".to_owned(),
        }
    );
}

#[test]
fn unsupported_canonical_source_type_is_not_coerced() {
    let array = RelationSchema::new(
        "t",
        vec![
            SchemaColumn::from_sql_type("items", "ARRAY<INTEGER>", "generic")
                .expect("array datatype should normalize"),
        ],
    )
    .expect("schema should be valid");

    let error = generate_from_sql("SELECT items FROM t", "generic", &[array], ROWS, SEED)
        .expect_err("array generation is not implemented");

    assert_eq!(
        error,
        ProtocolGenerationError::UnsupportedSourceType {
            relation: "t".to_owned(),
            column: "items".to_owned(),
            data_type: DataType::Array {
                element: Some(Box::new(DataType::SignedInteger { bits: Some(32) })),
                length: None,
            }
            .kind()
            .to_owned(),
        }
    );
}
