use sql_tdg::{
    GenerationBoundary, OutcomeSelector, ProtocolGenerationError, RelationSchema, SchemaColumn,
    generate_from_sql, generate_from_sql_at_boundary,
};

const ROWS: usize = 8;
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

fn assert_local_relation_error(error: &ProtocolGenerationError, relation: &str) {
    let ProtocolGenerationError::UnsupportedSemantics { code, message, .. } = error else {
        panic!("expected UnsupportedSemantics, got {error:?}");
    };
    assert_eq!(code, "local_relation_semantics");
    assert!(message.contains(relation), "message: {message}");
}

#[test]
fn cte_source_is_refused_instead_of_dropping_its_filter() {
    let error = generate_from_sql(
        "WITH big AS (SELECT a FROM t WHERE a > 1000) SELECT a FROM big",
        "generic",
        &[schema("t", &[("a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("CTE semantics are not carried by the protocol");

    assert_local_relation_error(&error, "big");
}

#[test]
fn derived_table_source_is_refused_instead_of_dropping_its_filter() {
    let error = generate_from_sql(
        "SELECT a FROM (SELECT a FROM t WHERE a > 1000) AS big",
        "generic",
        &[schema("t", &[("a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("derived-table semantics are not carried by the protocol");

    let ProtocolGenerationError::UnsupportedSemantics { code, .. } = &error else {
        panic!("expected UnsupportedSemantics, got {error:?}");
    };
    assert_eq!(code, "local_relation_semantics");
}

#[test]
fn ancestor_layer_reading_a_cte_is_refused() {
    let sql = "
        CREATE VIEW stage AS WITH big AS (SELECT a FROM t WHERE a > 1000) SELECT a FROM big;
        CREATE VIEW final AS SELECT a FROM stage WHERE a < 5000;
    ";
    let schemas = [
        schema("t", &[("a", "INTEGER")]),
        schema("stage", &[("a", "INTEGER")]),
    ];

    let physical = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect_err("ancestor CTE semantics are not carried by the protocol");
    assert_local_relation_error(&physical, "big");

    let boundary =
        GenerationBoundary::intermediate_relations(["stage"]).expect("boundary should build");
    let intermediate = generate_from_sql_at_boundary(
        sql,
        "generic",
        &schemas,
        Some(&OutcomeSelector::Relation("final".to_owned())),
        &boundary,
        ROWS,
        SEED,
    )
    .expect_err("intermediate generation must not rely on dropped CTE semantics");
    assert_local_relation_error(&intermediate, "big");
}

#[test]
fn whole_project_names_the_outcome_reading_a_cte() {
    let sql = "
        CREATE VIEW plain AS SELECT a FROM t WHERE a >= 10;
        CREATE VIEW nested AS WITH big AS (SELECT a FROM t WHERE a > 1000) SELECT a FROM big;
    ";

    let error = generate_from_sql(
        sql,
        "generic",
        &[schema("t", &[("a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect_err("one outcome reading a CTE must fail the shared generation");

    let ProtocolGenerationError::TerminalOutcome { outcome, source } = &error else {
        panic!("expected TerminalOutcome, got {error:?}");
    };
    assert_eq!(outcome, "relation:nested");
    assert_local_relation_error(source, "big");
}

#[test]
fn physical_and_produced_sources_are_unaffected() {
    let sql = "
        CREATE VIEW stage AS SELECT a FROM t WHERE a >= 10;
        CREATE VIEW final AS SELECT a FROM stage WHERE a <= 20;
    ";
    let schemas = [
        schema("t", &[("a", "INTEGER")]),
        schema("stage", &[("a", "INTEGER")]),
    ];

    let generated = generate_from_sql(sql, "generic", &schemas, ROWS, SEED)
        .expect("physical and produced relations remain supported");
    let values = generated
        .table("t")
        .expect("source should be generated")
        .get_ints("a")
        .expect("column should be readable")
        .expect("column should be built");
    assert!(values.iter().all(|value| (10..=20).contains(value)));
}
