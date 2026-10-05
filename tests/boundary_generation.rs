use sql_tdg::{
    GenerationBoundary, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError,
    RelationSchema, SchemaColumn, generate_classified_from_sql_at_boundary,
};

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
    .expect("test schema should be valid")
}

#[test]
fn intermediate_boundary_materializes_only_direct_upstream_relation() {
    let sql = "
        CREATE VIEW stage_orders AS
        SELECT amount, customer_id
        FROM raw_orders
        WHERE amount >= 10;

        CREATE VIEW final_orders AS
        SELECT customer_id
        FROM stage_orders
        WHERE amount <= 20;
    ";
    let schemas = [
        schema(
            "raw_orders",
            &[("amount", "INTEGER"), ("customer_id", "INTEGER")],
        ),
        schema(
            "stage_orders",
            &[("amount", "INTEGER"), ("customer_id", "INTEGER")],
        ),
    ];
    let boundary =
        GenerationBoundary::intermediate_relations(["stage_orders"]).expect("boundary should build");
    let counts = GenerationRowCounts::new(4, 6).expect("row counts should be valid");

    let generated = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        Some(&OutcomeSelector::Relation("final_orders".to_owned())),
        &boundary,
        counts,
        SEED,
    )
    .expect("intermediate generation should succeed");

    assert_eq!(generated.tables().len(), 1);
    assert!(generated.table("raw_orders").is_none());
    let stage = generated
        .table("stage_orders")
        .expect("selected intermediate relation should be materialized");
    let amounts = stage
        .get_ints("amount")
        .expect("amount should be readable")
        .expect("amount should be built");

    assert!(
        amounts[..counts.matching()]
            .iter()
            .all(|value| (10..=20).contains(value))
    );
    assert!(
        amounts[counts.matching()..]
            .iter()
            .all(|value| *value >= 10 && *value > 20)
    );
}

#[test]
fn intermediate_boundary_preserves_relational_classification() {
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
        JOIN stage_customers AS c ON o.customer_id = c.id
        WHERE o.amount <= 20;
    ";
    let schemas = [
        schema(
            "raw_orders",
            &[("customer_id", "INTEGER"), ("amount", "INTEGER")],
        ),
        schema("raw_customers", &[("id", "INTEGER"), ("active", "BOOLEAN")]),
        schema(
            "stage_orders",
            &[("customer_id", "INTEGER"), ("amount", "INTEGER")],
        ),
        schema("stage_customers", &[("id", "INTEGER"), ("active", "BOOLEAN")]),
    ];
    let boundary = GenerationBoundary::intermediate_relations([
        "stage_orders",
        "stage_customers",
    ])
    .expect("boundary should build");
    let counts = GenerationRowCounts::new(5, 7).expect("row counts should be valid");

    let generated = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        Some(&OutcomeSelector::Relation("final_orders".to_owned())),
        &boundary,
        counts,
        SEED,
    )
    .expect("relational intermediate generation should succeed");

    assert_eq!(
        generated.tables().keys().cloned().collect::<Vec<_>>(),
        vec!["stage_customers".to_owned(), "stage_orders".to_owned()]
    );
    let orders = generated
        .table("stage_orders")
        .expect("stage_orders should be materialized");
    let customers = generated
        .table("stage_customers")
        .expect("stage_customers should be materialized");
    let order_customer_ids = orders
        .get_ints("customer_id")
        .expect("customer_id should be readable")
        .expect("customer_id should be built");
    let customer_ids = customers
        .get_ints("id")
        .expect("id should be readable")
        .expect("id should be built");
    let amounts = orders
        .get_ints("amount")
        .expect("amount should be readable")
        .expect("amount should be built");
    let active = customers
        .get_bools("active")
        .expect("active should be readable")
        .expect("active should be built");

    for index in 0..counts.matching() {
        assert_eq!(order_customer_ids[index], customer_ids[index]);
    }
    for index in counts.matching()..counts.total() {
        assert_ne!(order_customer_ids[index], customer_ids[index]);
    }
    assert!(amounts.iter().all(|value| (10..=20).contains(value)));
    assert!(active.iter().all(|value| *value));
}

#[test]
fn intermediate_boundary_must_cover_every_direct_target_input() {
    let sql = "
        CREATE VIEW stage_orders AS
        SELECT customer_id FROM raw_orders;

        CREATE VIEW stage_customers AS
        SELECT id FROM raw_customers;

        CREATE VIEW final_orders AS
        SELECT o.customer_id
        FROM stage_orders AS o
        JOIN stage_customers AS c ON o.customer_id = c.id;
    ";
    let schemas = [
        schema("raw_orders", &[("customer_id", "INTEGER")]),
        schema("raw_customers", &[("id", "INTEGER")]),
        schema("stage_orders", &[("customer_id", "INTEGER")]),
        schema("stage_customers", &[("id", "INTEGER")]),
    ];
    let boundary =
        GenerationBoundary::intermediate_relations(["stage_orders"]).expect("boundary should build");

    let error = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        Some(&OutcomeSelector::Relation("final_orders".to_owned())),
        &boundary,
        GenerationRowCounts::matching_only(4),
        SEED,
    )
    .expect_err("partial intermediate cut must fail");

    assert!(matches!(
        error,
        ProtocolGenerationError::InvalidGenerationBoundary { .. }
    ));
}

#[test]
fn ambiguous_terminal_components_still_require_explicit_target_selection() {
    let sql = "
        CREATE VIEW stage_a AS SELECT value FROM raw_a;
        CREATE VIEW final_a AS SELECT value FROM stage_a;
        CREATE VIEW stage_b AS SELECT value FROM raw_b;
        CREATE VIEW final_b AS SELECT value FROM stage_b;
    ";
    let schemas = [
        schema("raw_a", &[("value", "INTEGER")]),
        schema("stage_a", &[("value", "INTEGER")]),
        schema("raw_b", &[("value", "INTEGER")]),
        schema("stage_b", &[("value", "INTEGER")]),
    ];
    let boundary =
        GenerationBoundary::intermediate_relations(["stage_a"]).expect("boundary should build");

    let error = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        None,
        &boundary,
        GenerationRowCounts::matching_only(4),
        SEED,
    )
    .expect_err("ambiguous components must require a target selector");

    assert!(matches!(
        error,
        ProtocolGenerationError::AmbiguousTerminalOutcome { .. }
    ));
}
