use sql_semantic_protocol::{
    AnalysisBundle, ConfiguredSqlInput, RelationCatalog, RelationSchema, SchemaColumn, SqlInput,
    analyze_configured_inputs_with_catalog, dialect_from_name,
};
use sql_tdg::{
    GeneratedData, ProtocolGenerationError, generate_from_sql, generate_scenarios_from_bundle,
};

fn bundle(sql: &str) -> AnalysisBundle {
    let dialect = dialect_from_name("generic").expect("generic dialect must exist");
    let input = SqlInput::inline(sql);
    let configured = [ConfiguredSqlInput::new(
        "scenarios",
        &input,
        "generic",
        dialect.as_ref(),
    )];
    let schemas = [
        RelationSchema::new(
            "orders",
            vec![
                SchemaColumn::from_sql_type("amount", "INTEGER", "generic")
                    .expect("valid amount type"),
                SchemaColumn::from_sql_type("id", "INTEGER", "generic").expect("valid id type"),
            ],
        )
        .expect("valid schema"),
        RelationSchema::new(
            "customers",
            vec![
                SchemaColumn::from_sql_type("id", "INTEGER", "generic")
                    .expect("valid customer type"),
            ],
        )
        .expect("valid schema"),
    ];
    let catalog = RelationCatalog::from_schemas(&schemas).expect("valid catalog");
    analyze_configured_inputs_with_catalog(&configured, &catalog).expect("valid analysis")
}

fn ints(data: &GeneratedData, relation: &str, column: &str) -> Vec<i32> {
    data.table(relation)
        .expect("expected generated relation")
        .get_ints(column)
        .expect("column can be read")
        .expect("expected integer column")
}

#[test]
fn compatible_terminal_outcomes_share_a_scenario_and_conflicts_split() {
    let sql = "
        CREATE VIEW high_orders AS SELECT amount FROM orders WHERE amount > 100;
        CREATE VIEW capped_orders AS SELECT amount FROM orders WHERE amount <= 200;
        CREATE VIEW low_orders AS SELECT amount FROM orders WHERE amount < 50;
        CREATE VIEW vip_customers AS SELECT id FROM customers WHERE id >= 5;
    ";
    let analyzed = bundle(sql);
    let scenarios =
        generate_scenarios_from_bundle(&analyzed, 12, 42).expect("compatible partition exists");
    assert_eq!(scenarios.len(), 2);
    assert_eq!(
        scenarios[0].outcomes(),
        &[
            "relation:capped_orders".to_owned(),
            "relation:high_orders".to_owned(),
            "relation:vip_customers".to_owned(),
        ]
    );
    assert_eq!(scenarios[1].outcomes(), &["relation:low_orders".to_owned()]);

    let high = ints(scenarios[0].data(), "orders", "amount");
    assert_eq!(high.len(), 12);
    assert!(high.iter().all(|value| (101..=200).contains(value)));
    assert!(
        ints(scenarios[0].data(), "customers", "id")
            .iter()
            .all(|id| *id >= 5)
    );
    assert!(
        ints(scenarios[1].data(), "orders", "amount")
            .iter()
            .all(|value| *value < 50)
    );
    assert!(scenarios[1].data().table("customers").is_none());

    let again = generate_scenarios_from_bundle(&analyzed, 12, 42).expect("repeat succeeds");
    for (first, second) in scenarios.iter().zip(&again) {
        assert_eq!(first.outcomes(), second.outcomes());
        for (name, table) in first.data().tables() {
            assert_eq!(
                table.dim().rows(),
                second.data().table(name).expect("same table").dim().rows()
            );
        }
        for name in first.data().tables().keys() {
            for column in ["amount", "id"] {
                if let Ok(Some(values)) = first.data().table(name).expect("table").get_ints(column)
                {
                    assert_eq!(
                        values,
                        second
                            .data()
                            .table(name)
                            .expect("table")
                            .get_ints(column)
                            .expect("read")
                            .expect("same column")
                    );
                }
            }
        }
    }

    let schemas = [
        RelationSchema::new(
            "orders",
            vec![SchemaColumn::from_sql_type("amount", "INTEGER", "generic").expect("type")],
        )
        .expect("schema"),
        RelationSchema::new(
            "customers",
            vec![SchemaColumn::from_sql_type("id", "INTEGER", "generic").expect("type")],
        )
        .expect("schema"),
    ];
    assert!(matches!(
        generate_from_sql(sql, "generic", &schemas, 12, 42),
        Err(ProtocolGenerationError::ConflictingOutcomes { .. })
    ));
}

#[test]
fn unsupported_individual_outcome_cannot_be_silently_skipped() {
    let analyzed = bundle(
        "
        CREATE VIEW supported AS SELECT amount FROM orders WHERE amount >= 10;
        CREATE VIEW unsupported AS
        SELECT o.amount FROM orders AS o
        WHERE EXISTS (SELECT 1 FROM customers AS c WHERE c.id = o.id);
    ",
    );
    let error = generate_scenarios_from_bundle(&analyzed, 8, 42)
        .expect_err("unsupported terminal outcome must fail closed");
    assert!(matches!(
        error,
        ProtocolGenerationError::TerminalOutcome { .. }
    ));
}
