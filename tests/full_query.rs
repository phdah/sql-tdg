use sql_semantic_protocol::{
    ConfiguredSqlInput, RelationCatalog, SqlInput, analyze_configured_inputs_with_catalog,
    dialect_from_name,
};
use sql_tdg::{
    DataType, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError, RelationSchema,
    SchemaColumn, generate_classified_from_sql, generate_from_bundle, generate_from_sql,
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
fn protocol_set_domain_is_consumed_directly() {
    let generated = generate_from_sql(
        "SELECT col_a FROM t WHERE col_a IN (1, 3, 5)",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect("set-domain generation should succeed");

    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert!(values.iter().all(|value| [1, 3, 5].contains(value)));
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
fn explicit_terminal_selection_generates_only_the_selected_outcome() {
    let schemas = [schema("t", &[("col_a", "INTEGER")])];
    let catalog = RelationCatalog::from_schemas(&schemas).expect("catalog should be valid");
    let input = SqlInput::inline(
        "SELECT col_a FROM t WHERE col_a = 10; SELECT col_a FROM t WHERE col_a = 20",
    );
    let dialect = dialect_from_name("generic").expect("generic dialect should exist");
    let configured = [ConfiguredSqlInput::new(
        "input-0001",
        &input,
        "generic",
        dialect.as_ref(),
    )];
    let bundle = analyze_configured_inputs_with_catalog(&configured, &catalog)
        .expect("protocol analysis should succeed");
    let selector = OutcomeSelector::AnonymousLayer("layer-0002".to_owned());

    let generated = generate_from_bundle(&bundle, Some(&selector), ROWS, SEED)
        .expect("explicit terminal selection should succeed");
    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert_eq!(values, vec![20; ROWS]);
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
fn constrained_column_missing_from_source_schema_is_an_explicit_error() {
    let error = generate_from_sql(
        "SELECT col_a FROM t WHERE required_col = 10",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("a constrained column cannot be silently omitted from schema");

    assert_eq!(
        error,
        ProtocolGenerationError::MissingSchemaColumn {
            relation: "t".to_owned(),
            column: "required_col".to_owned(),
        }
    );
}

#[test]
fn canonical_array_source_type_is_generated_losslessly() {
    let array = RelationSchema::new(
        "t",
        vec![
            SchemaColumn::from_sql_type("items", "ARRAY<INTEGER>", "generic")
                .expect("array datatype should normalize"),
        ],
    )
    .expect("schema should be valid");

    let generated = generate_from_sql("SELECT items FROM t", "generic", &[array], ROWS, SEED)
        .expect("array generation should succeed");
    let table = generated.table("t").expect("source table should exist");
    let values = table
        .array("items")
        .expect("array column should be readable")
        .expect("array column should be built");

    assert_eq!(
        values.data_type(),
        &arrow_schema::DataType::List(std::sync::Arc::new(arrow_schema::Field::new_list_field(
            arrow_schema::DataType::Int32,
            false
        )))
    );
    assert_eq!(values.len(), ROWS);
}

#[test]
fn unsupported_custom_source_type_is_not_coerced() {
    let custom = RelationSchema::new(
        "t",
        vec![
            SchemaColumn::new(
                "value",
                DataType::Custom {
                    name: "vendor_money".to_owned(),
                    modifiers: vec!["42".to_owned()],
                },
            )
            .expect("custom datatype should be valid schema metadata"),
        ],
    )
    .expect("schema should be valid");

    let error = generate_from_sql("SELECT value FROM t", "generic", &[custom], ROWS, SEED)
        .expect_err("unsupported custom type must fail explicitly");

    assert_eq!(
        error,
        ProtocolGenerationError::UnsupportedSourceType {
            relation: "t".to_owned(),
            column: "value".to_owned(),
            data_type: "custom:vendor_money(42)".to_owned(),
        }
    );
}


#[test]
fn classified_generation_produces_matching_and_rejected_range_witnesses() {
    let counts = GenerationRowCounts::new(5, 7).expect("test row counts should be valid");
    let generated = generate_classified_from_sql(
        "SELECT col_a FROM t WHERE col_a >= 4 AND col_a < 8 AND col_a != 6",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        counts,
        SEED,
    )
    .expect("classified range generation should succeed");

    assert_eq!(generated.row_counts(), counts);
    let values = generated
        .table("t")
        .expect("source table should exist")
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");
    let (matching, rejected) = values.split_at(counts.matching());

    assert!(
        matching
            .iter()
            .all(|value| (4..8).contains(value) && *value != 6)
    );
    assert!(
        rejected
            .iter()
            .all(|value| !(4..8).contains(value) || *value == 6)
    );
}

#[test]
fn rejected_column_selection_and_values_are_seeded() {
    let counts = GenerationRowCounts::new(4, 12).expect("test row counts should be valid");
    let schemas = [schema(
        "t",
        &[("col_a", "INTEGER"), ("enabled", "BOOLEAN")],
    )];
    let sql = "SELECT col_a, enabled FROM t WHERE col_a BETWEEN 4 AND 8 AND enabled = true";

    let first = generate_classified_from_sql(sql, "generic", &schemas, counts, SEED)
        .expect("classified generation should succeed");
    let second = generate_classified_from_sql(sql, "generic", &schemas, counts, SEED)
        .expect("classified generation should repeat");

    let first_table = first.table("t").expect("source table should exist");
    let second_table = second.table("t").expect("source table should exist");
    let first_ints = first_table
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");
    let second_ints = second_table
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");
    let first_bools = first_table
        .get_bools("enabled")
        .expect("boolean column should be readable")
        .expect("boolean column should be built");
    let second_bools = second_table
        .get_bools("enabled")
        .expect("boolean column should be readable")
        .expect("boolean column should be built");

    assert_eq!(first_ints, second_ints);
    assert_eq!(first_bools, second_bools);

    for (value, enabled) in first_ints[counts.matching()..]
        .iter()
        .zip(&first_bools[counts.matching()..])
    {
        assert!(!(4..=8).contains(value) || !enabled);
    }
}

#[test]
fn finite_string_and_boolean_domains_have_rejected_witnesses() {
    let counts = GenerationRowCounts::new(3, 5).expect("test row counts should be valid");

    let strings = generate_classified_from_sql(
        "SELECT status FROM t WHERE status IN ('ready', 'done')",
        "generic",
        &[schema("t", &[("status", "TEXT")])],
        counts,
        SEED,
    )
    .expect("finite string domain should be complementable");
    let statuses = strings
        .table("t")
        .expect("source table should exist")
        .get_strings("status")
        .expect("string column should be readable")
        .expect("string column should be built");
    let (matching, rejected) = statuses.split_at(counts.matching());

    assert!(
        matching
            .iter()
            .all(|value| value == "ready" || value == "done")
    );
    assert!(
        rejected
            .iter()
            .all(|value| value != "ready" && value != "done")
    );

    let booleans = generate_classified_from_sql(
        "SELECT enabled FROM t WHERE enabled = true",
        "generic",
        &[schema("t", &[("enabled", "BOOLEAN")])],
        counts,
        SEED,
    )
    .expect("boolean domain should be complementable");
    let enabled = booleans
        .table("t")
        .expect("source table should exist")
        .get_bools("enabled")
        .expect("boolean column should be readable")
        .expect("boolean column should be built");

    assert!(enabled[..counts.matching()].iter().all(|value| *value));
    assert!(enabled[counts.matching()..].iter().all(|value| !*value));
}

#[test]
fn nullable_domain_can_use_null_as_a_rejected_witness() {
    let nullable_int = RelationSchema::new(
        "t",
        vec![
            SchemaColumn::new(
                "value",
                DataType::Nullable(Box::new(DataType::SignedInteger { bits: Some(32) })),
            )
            .expect("nullable datatype should be valid schema metadata"),
        ],
    )
    .expect("schema should be valid");
    let counts = GenerationRowCounts::new(2, 4).expect("test row counts should be valid");

    let generated = generate_classified_from_sql(
        "SELECT value FROM t WHERE value IS NOT NULL",
        "generic",
        &[nullable_int],
        counts,
        SEED,
    )
    .expect("nullable domain should be complementable");
    let array = generated
        .table("t")
        .expect("source table should exist")
        .array("value")
        .expect("column should be readable")
        .expect("column should be built");

    assert!((0..counts.matching()).all(|index| !array.is_null(index)));
    assert!((counts.matching()..counts.total()).all(|index| array.is_null(index)));
}

#[test]
fn unbounded_domain_cannot_claim_rejected_rows() {
    let counts = GenerationRowCounts::new(1, 1).expect("test row counts should be valid");
    let error = generate_classified_from_sql(
        "SELECT col_a FROM t",
        "generic",
        &[schema("t", &[("col_a", "INTEGER")])],
        counts,
        SEED,
    )
    .expect_err("an unbounded domain has no guaranteed rejection witness");

    assert_eq!(
        error,
        ProtocolGenerationError::NoRejectableColumn {
            relation: "t".to_owned(),
        }
    );
}

#[test]
fn rejected_ratio_resolves_to_deterministic_counts() {
    let counts =
        GenerationRowCounts::from_rejected_ratio(10, 0.3).expect("ratio should be valid");

    assert_eq!(counts.matching(), 7);
    assert_eq!(counts.rejected(), 3);
    assert_eq!(counts.total(), 10);
    assert!(GenerationRowCounts::from_rejected_ratio(10, 1.1).is_err());
}
