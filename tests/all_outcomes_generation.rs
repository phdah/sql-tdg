use sql_tdg::{
    GenerationBoundary, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError,
    RelationSchema, SchemaColumn, generate_classified_from_sql_at_boundary, generate_from_sql,
};

const ROWS: usize = 24;
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

fn ints(generated: &sql_tdg::GeneratedData, relation: &str, column: &str) -> Vec<i32> {
    generated
        .table(relation)
        .unwrap_or_else(|| panic!("relation {relation} should be generated"))
        .get_ints(column)
        .expect("integer column should be readable")
        .expect("integer column should be built")
}

#[test]
fn all_outcomes_generate_every_disjoint_source_once() {
    let sql = "
        CREATE VIEW final_a AS SELECT amount FROM raw_a WHERE amount >= 10;
        CREATE VIEW final_b AS SELECT amount FROM raw_b WHERE amount < 0;
    ";
    let schemas = [
        schema("raw_a", &[("amount", "INTEGER")]),
        schema("raw_b", &[("amount", "INTEGER")]),
    ];

    let generated = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("all terminal outcomes should generate one dataset");

    assert_eq!(
        generated.tables().keys().collect::<Vec<_>>(),
        ["raw_a", "raw_b"]
    );
    let a = ints(&generated, "raw_a", "amount");
    let b = ints(&generated, "raw_b", "amount");
    assert_eq!(a.len(), ROWS);
    assert_eq!(b.len(), ROWS);
    assert!(a.iter().all(|value| *value >= 10));
    assert!(b.iter().all(|value| *value < 0));
}

#[test]
fn shared_source_rows_satisfy_every_dependent_outcome() {
    let sql = "
        CREATE VIEW high_orders AS SELECT amount, status FROM orders WHERE amount >= 10;
        CREATE VIEW capped_orders AS SELECT amount FROM orders WHERE amount <= 20;
        CREATE VIEW open_orders AS SELECT status FROM orders WHERE status IN (1, 2);
    ";
    let schemas = [schema(
        "orders",
        &[("amount", "INTEGER"), ("status", "INTEGER")],
    )];

    let generated = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("compatible shared-source outcomes should generate");

    assert_eq!(generated.tables().len(), 1);
    let amounts = ints(&generated, "orders", "amount");
    let statuses = ints(&generated, "orders", "status");
    assert_eq!(amounts.len(), ROWS);
    assert!(amounts.iter().all(|value| (10..=20).contains(value)));
    assert!(statuses.iter().all(|value| [1, 2].contains(value)));
}

#[test]
fn contradictory_outcomes_on_a_shared_source_name_the_outcomes() {
    let sql = "
        CREATE VIEW big_orders AS SELECT amount FROM orders WHERE amount > 100;
        CREATE VIEW small_orders AS SELECT amount FROM orders WHERE amount < 50;
    ";
    let schemas = [schema("orders", &[("amount", "INTEGER")])];

    let error = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect_err("contradictory outcomes must not generate a shared dataset");

    let ProtocolGenerationError::ConflictingOutcomes { outcomes, message } = &error else {
        panic!("expected ConflictingOutcomes, got {error:?}");
    };
    assert_eq!(
        outcomes,
        &[
            "relation:big_orders".to_owned(),
            "relation:small_orders".to_owned()
        ]
    );
    assert!(message.contains("orders.amount"), "message: {message}");
    assert!(error.to_string().contains("relation:big_orders"));
}

#[test]
fn shared_relationship_keys_satisfy_every_outcome() {
    let sql = "
        CREATE VIEW joined AS
        SELECT o.amount
        FROM orders AS o
        INNER JOIN customers AS c ON o.customer_id = c.id;

        CREATE VIEW vip_orders AS
        SELECT customer_id FROM orders WHERE customer_id >= 5;

        CREATE VIEW low_customers AS
        SELECT id FROM customers WHERE id <= 7;
    ";
    let schemas = [
        schema(
            "orders",
            &[("amount", "INTEGER"), ("customer_id", "INTEGER")],
        ),
        schema("customers", &[("id", "INTEGER")]),
    ];

    let generated = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("compatible relationship outcomes should generate");

    let customer_ids = ints(&generated, "orders", "customer_id");
    let ids = ints(&generated, "customers", "id");
    assert!(customer_ids.iter().all(|value| (5..=7).contains(value)));
    assert!(ids.iter().all(|value| (5..=7).contains(value)));
    assert!(
        customer_ids.iter().all(|value| ids.contains(value)),
        "every order must join to a generated customer"
    );
}

#[test]
fn contradictory_relationship_outcomes_name_the_outcomes() {
    let sql = "
        CREATE VIEW joined AS
        SELECT o.amount
        FROM orders AS o
        INNER JOIN customers AS c ON o.customer_id = c.id;

        CREATE VIEW vip_orders AS
        SELECT customer_id FROM orders WHERE customer_id >= 100;

        CREATE VIEW low_customers AS
        SELECT id FROM customers WHERE id <= 7;
    ";
    let schemas = [
        schema(
            "orders",
            &[("amount", "INTEGER"), ("customer_id", "INTEGER")],
        ),
        schema("customers", &[("id", "INTEGER")]),
    ];

    let error = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect_err("relationship keys with no common value must fail");

    let ProtocolGenerationError::ConflictingOutcomes { outcomes, .. } = &error else {
        panic!("expected ConflictingOutcomes, got {error:?}");
    };
    assert_eq!(
        outcomes,
        &[
            "relation:joined".to_owned(),
            "relation:low_customers".to_owned(),
            "relation:vip_orders".to_owned(),
        ]
    );
}

#[test]
fn unsupported_outcome_is_named_instead_of_dropped() {
    let sql = "
        CREATE VIEW supported AS SELECT amount FROM orders WHERE amount >= 10;
        CREATE VIEW correlated AS
        SELECT o.amount FROM orders AS o
        WHERE EXISTS (SELECT 1 FROM customers AS c WHERE c.id = o.customer_id);
    ";
    let schemas = [
        schema(
            "orders",
            &[("amount", "INTEGER"), ("customer_id", "INTEGER")],
        ),
        schema("customers", &[("id", "INTEGER")]),
    ];

    let error = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect_err("an unsupported outcome must fail the shared generation");

    let ProtocolGenerationError::TerminalOutcome { outcome, .. } = &error else {
        panic!("expected TerminalOutcome, got {error:?}");
    };
    assert_eq!(outcome, "relation:correlated");
    assert!(error.to_string().contains("EXISTS"), "error: {error}");
}

#[test]
fn all_outcomes_reject_deliberately_rejected_rows() {
    let sql = "
        CREATE VIEW final_a AS SELECT amount FROM orders WHERE amount >= 10;
        CREATE VIEW final_b AS SELECT amount FROM orders WHERE amount <= 20;
    ";
    let schemas = [schema("orders", &[("amount", "INTEGER")])];
    let counts = GenerationRowCounts::new(4, 2).expect("row counts should be valid");

    let error = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        None,
        &GenerationBoundary::physical_sources(),
        counts,
        SEED,
    )
    .expect_err("rejected rows are undefined across several outcomes");

    assert!(matches!(
        error,
        ProtocolGenerationError::InvalidRowConfiguration { .. }
    ));
}

#[test]
fn all_outcomes_generation_is_deterministic() {
    let sql = "
        CREATE VIEW final_a AS SELECT amount FROM orders WHERE amount >= 10;
        CREATE VIEW final_b AS SELECT amount, status FROM orders WHERE amount <= 20 AND status > 0;
    ";
    let schemas = [schema(
        "orders",
        &[("amount", "INTEGER"), ("status", "INTEGER")],
    )];

    let first = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("first generation should succeed");
    let second = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("second generation should succeed");

    assert_eq!(
        ints(&first, "orders", "amount"),
        ints(&second, "orders", "amount")
    );
    assert_eq!(
        ints(&first, "orders", "status"),
        ints(&second, "orders", "status")
    );
}

#[test]
fn explicit_selector_still_generates_only_the_selected_outcome() {
    let sql = "
        CREATE VIEW big_orders AS SELECT amount FROM orders WHERE amount > 100;
        CREATE VIEW small_orders AS SELECT amount FROM orders WHERE amount < 50;
    ";
    let schemas = [schema("orders", &[("amount", "INTEGER")])];

    let generated = generate_classified_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        Some(&OutcomeSelector::Relation("small_orders".to_owned())),
        &GenerationBoundary::physical_sources(),
        GenerationRowCounts::matching_only(ROWS),
        SEED,
    )
    .expect("explicit selection should ignore other outcomes");

    assert!(
        ints(&generated, "orders", "amount")
            .iter()
            .all(|value| *value < 50)
    );
}
