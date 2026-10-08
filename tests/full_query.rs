use sql_semantic_protocol::{
    ComparisonAssumption, ConfiguredSqlInput, RelationCatalog, SqlInput,
    analyze_configured_inputs_with_catalog, dialect_from_name,
};
use sql_tdg::{
    DataType, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError, RelationSchema,
    SchemaColumn, generate_classified_from_sql, generate_classified_from_sql_with_assumptions,
    generate_from_bundle, generate_from_sql, to_timestamp,
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
    let generated = generate_classified_from_sql_with_assumptions(
        &format!("SELECT created_at FROM t WHERE created_at = TIMESTAMP '{literal}'"),
        "generic",
        &[schema("t", &[("created_at", "TIMESTAMP")])],
        GenerationRowCounts::matching_only(ROWS),
        SEED,
        &[ComparisonAssumption::SessionTimeZone],
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

    assert!(matches!(
        error,
        ProtocolGenerationError::ResidualConditions { .. }
    ));
    assert!(error.to_string().contains("reason=unknown_schema_column"));
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
    let schemas = [schema("t", &[("col_a", "INTEGER"), ("enabled", "BOOLEAN")])];
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

    let strings = generate_classified_from_sql_with_assumptions(
        "SELECT status FROM t WHERE status IN ('ready', 'done')",
        "generic",
        &[schema("t", &[("status", "TEXT")])],
        counts,
        SEED,
        &[ComparisonAssumption::BinaryCollation],
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
    let counts = GenerationRowCounts::from_rejected_ratio(10, 0.3).expect("ratio should be valid");

    assert_eq!(counts.matching(), 7);
    assert_eq!(counts.rejected(), 3);
    assert_eq!(counts.total(), 10);
    assert!(GenerationRowCounts::from_rejected_ratio(10, 1.1).is_err());
}

#[test]
fn relational_generation_coordinates_inner_join_keys_and_breaks_one_relationship() {
    let counts = GenerationRowCounts::new(5, 7).expect("test row counts should be valid");
    let schemas = [
        schema(
            "orders",
            &[("customer_id", "INTEGER"), ("amount", "INTEGER")],
        ),
        schema("customers", &[("id", "INTEGER"), ("active", "BOOLEAN")]),
    ];
    let generated = generate_classified_from_sql(
        "SELECT o.customer_id, o.amount
         FROM orders AS o
         JOIN customers AS c ON o.customer_id = c.id
         WHERE o.amount BETWEEN 10 AND 20 AND c.active = true",
        "generic",
        &schemas,
        counts,
        SEED,
    )
    .expect("relational generation should succeed");

    let order_customer_ids = generated
        .table("orders")
        .expect("orders should exist")
        .get_ints("customer_id")
        .expect("customer_id should be readable")
        .expect("customer_id should be built");
    let customer_ids = generated
        .table("customers")
        .expect("customers should exist")
        .get_ints("id")
        .expect("id should be readable")
        .expect("id should be built");
    let amounts = generated
        .table("orders")
        .expect("orders should exist")
        .get_ints("amount")
        .expect("amount should be readable")
        .expect("amount should be built");
    let active = generated
        .table("customers")
        .expect("customers should exist")
        .get_bools("active")
        .expect("active should be readable")
        .expect("active should be built");

    for index in 0..counts.matching() {
        assert_eq!(order_customer_ids[index], customer_ids[index]);
    }
    let matching_keys = order_customer_ids
        .iter()
        .take(counts.matching())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(matching_keys.len() >= 3, "join keys should be distributed");
    assert!(
        matching_keys.iter().all(|key| (1..=1000).contains(key)),
        "unconstrained join keys should be moderate: {matching_keys:?}"
    );
    for index in counts.matching()..counts.total() {
        assert_ne!(order_customer_ids[index], customer_ids[index]);
    }
    assert!(amounts.iter().all(|amount| (10..=20).contains(amount)));
    assert!(active.iter().all(|value| *value));
}

#[test]
fn relational_generation_resolves_intermediate_join_keys_to_physical_sources() {
    let counts = GenerationRowCounts::new(4, 6).expect("test row counts should be valid");
    let sql = "
        CREATE VIEW stage_orders AS
        SELECT customer_id, amount
        FROM raw_orders
        WHERE amount >= 10;

        CREATE VIEW stage_customers AS
        SELECT id, active
        FROM raw_customers
        WHERE active = true;

        CREATE VIEW final_orders AS
        SELECT o.customer_id
        FROM stage_orders AS o
        JOIN stage_customers AS c ON o.customer_id = c.id;
    ";
    let schemas = [
        schema(
            "raw_orders",
            &[("customer_id", "INTEGER"), ("amount", "INTEGER")],
        ),
        schema("raw_customers", &[("id", "INTEGER"), ("active", "BOOLEAN")]),
    ];
    let generated = generate_classified_from_sql(sql, "generic", &schemas, counts, SEED)
        .expect("intermediate relationship should resolve through protocol lineage");

    let order_customer_ids = generated
        .table("raw_orders")
        .expect("raw_orders should exist")
        .get_ints("customer_id")
        .expect("customer_id should be readable")
        .expect("customer_id should be built");
    let customer_ids = generated
        .table("raw_customers")
        .expect("raw_customers should exist")
        .get_ints("id")
        .expect("id should be readable")
        .expect("id should be built");
    let amounts = generated
        .table("raw_orders")
        .expect("raw_orders should exist")
        .get_ints("amount")
        .expect("amount should be readable")
        .expect("amount should be built");
    let active = generated
        .table("raw_customers")
        .expect("raw_customers should exist")
        .get_bools("active")
        .expect("active should be readable")
        .expect("active should be built");

    for index in 0..counts.matching() {
        assert_eq!(order_customer_ids[index], customer_ids[index]);
    }
    let matching_keys = order_customer_ids
        .iter()
        .take(counts.matching())
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(matching_keys.len() >= 3, "join keys should be distributed");
    assert!(
        matching_keys.iter().all(|key| (1..=1000).contains(key)),
        "unconstrained join keys should be moderate: {matching_keys:?}"
    );
    for index in counts.matching()..counts.total() {
        assert_ne!(order_customer_ids[index], customer_ids[index]);
    }
    assert!(amounts.iter().all(|amount| *amount >= 10));
    assert!(active.iter().all(|value| *value));
}

#[test]
fn cte_join_keys_are_distributed_and_seeded() {
    let counts = GenerationRowCounts::new(8, 4).expect("valid row counts");
    let schemas = [
        schema("orders", &[("customer_id", "INTEGER"), ("amount", "INTEGER")]),
        schema("customers", &[("id", "INTEGER"), ("active", "BOOLEAN")]),
    ];
    let sql = "
        WITH filtered_orders AS (
            SELECT customer_id, amount FROM orders WHERE amount >= 10
        ), filtered_customers AS (
            SELECT id FROM customers WHERE active = true
        )
        SELECT o.customer_id
        FROM filtered_orders AS o
        JOIN filtered_customers AS c ON o.customer_id = c.id
    ";
    let generate = || {
        generate_classified_from_sql(sql, "generic", &schemas, counts, SEED)
            .expect("CTE join should generate")
    };
    let first = generate();
    let second = generate();
    let orders = first.table("orders").expect("orders");
    let customers = first.table("customers").expect("customers");
    let child_keys = orders.get_ints("customer_id").expect("keys").expect("built");
    let parent_keys = customers.get_ints("id").expect("keys").expect("built");
    let replay = second.table("orders").expect("replayed orders")
        .get_ints("customer_id").expect("keys").expect("built");
    assert_eq!(child_keys, replay, "the seed must reproduce identical keys");
    for row in 0..counts.matching() {
        assert_eq!(child_keys[row], parent_keys[row]);
    }
    for row in counts.matching()..counts.total() {
        assert_ne!(child_keys[row], parent_keys[row]);
    }
    let distinct = child_keys[..counts.matching()].iter().copied()
        .collect::<std::collections::BTreeSet<_>>();
    assert!(distinct.len() >= 3);
    assert!(distinct.iter().all(|key| (1..=1000).contains(key)));
}

#[test]
fn composite_join_equalities_are_coordinated_together() {
    let counts = GenerationRowCounts::new(3, 5).expect("test row counts should be valid");
    let schemas = [
        schema(
            "orders",
            &[("customer_id", "INTEGER"), ("region_id", "INTEGER")],
        ),
        schema("customers", &[("id", "INTEGER"), ("region_id", "INTEGER")]),
    ];
    let generated = generate_classified_from_sql(
        "SELECT o.customer_id
         FROM orders AS o
         JOIN customers AS c
           ON o.customer_id = c.id AND o.region_id = c.region_id",
        "generic",
        &schemas,
        counts,
        SEED,
    )
    .expect("composite equality relationship should generate");

    let orders = generated.table("orders").expect("orders should exist");
    let customers = generated
        .table("customers")
        .expect("customers should exist");
    let order_customer_ids = orders
        .get_ints("customer_id")
        .expect("customer_id should be readable")
        .expect("customer_id should be built");
    let order_regions = orders
        .get_ints("region_id")
        .expect("region_id should be readable")
        .expect("region_id should be built");
    let customer_ids = customers
        .get_ints("id")
        .expect("id should be readable")
        .expect("id should be built");
    let customer_regions = customers
        .get_ints("region_id")
        .expect("region_id should be readable")
        .expect("region_id should be built");

    for index in 0..counts.matching() {
        assert_eq!(order_customer_ids[index], customer_ids[index]);
        assert_eq!(order_regions[index], customer_regions[index]);
    }
    for index in counts.matching()..counts.total() {
        assert!(
            order_customer_ids[index] != customer_ids[index]
                || order_regions[index] != customer_regions[index]
        );
    }
}

#[test]
fn non_equality_relationship_is_an_explicit_error() {
    let error = generate_from_sql(
        "SELECT o.customer_id
         FROM orders AS o
         JOIN customers AS c ON o.customer_id < c.id",
        "generic",
        &[
            schema("orders", &[("customer_id", "INTEGER")]),
            schema("customers", &[("id", "INTEGER")]),
        ],
        ROWS,
        SEED,
    )
    .expect_err("non-equality relationships must not be approximated");

    assert!(matches!(
        error,
        ProtocolGenerationError::ResidualConditions { .. }
    ));
    assert!(error.to_string().contains("reason=column_comparison"));
}

#[test]
fn relational_subquery_predicates_are_explicit_errors() {
    let schemas = [
        schema("orders", &[("id", "INTEGER"), ("customer_id", "INTEGER")]),
        schema("line_items", &[("order_id", "INTEGER")]),
        schema("customers", &[("id", "INTEGER")]),
    ];

    for sql in [
        "SELECT o.id
         FROM orders AS o
         WHERE EXISTS (
             SELECT 1 FROM line_items AS li WHERE li.order_id = o.id
         )",
        "SELECT o.id
         FROM orders AS o
         WHERE o.customer_id IN (
             SELECT c.id FROM customers AS c
         )",
    ] {
        let error = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
            .expect_err("unsupported relational subquery must not be ignored");

        assert!(matches!(
            error,
            ProtocolGenerationError::ResidualConditions { .. }
        ));
        assert!(error.to_string().contains("reason=subquery_predicate"));
    }
}
