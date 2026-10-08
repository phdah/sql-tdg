use sql_semantic_protocol::ComparisonAssumption;
use sql_tdg::{
    GenerationBoundary, GenerationRowCounts, OutcomeSelector, ProtocolGenerationError,
    RelationSchema, SchemaColumn, SqlGenerationSettings,
    generate_classified_from_sql_at_boundary_with_assumptions,
    generate_classified_from_sql_with_assumptions,
};

fn schema(relation: &str, columns: &[(&str, &str)], dialect: &str) -> RelationSchema {
    RelationSchema::new(
        relation,
        columns
            .iter()
            .map(|(name, data_type)| {
                SchemaColumn::from_sql_type(*name, data_type, dialect)
                    .expect("fixture column type must be valid")
            })
            .collect(),
    )
    .expect("fixture schema must be valid")
}

#[test]
fn residual_conditions_fail_with_reason_clause_and_origin() {
    let cases = [
        (
            "SELECT a, b FROM t WHERE a = 1 OR b = 2",
            "cross_column_disjunction",
            "where",
            "generic",
        ),
        (
            "SELECT name FROM t WHERE name LIKE 'x%'",
            "computed_expression",
            "where",
            "generic",
        ),
        (
            "SELECT a FROM t WHERE CAST(a AS INT) > 5",
            "computed_expression",
            "where",
            "generic",
        ),
        (
            "SELECT a FROM t GROUP BY a HAVING COUNT(*) > 2",
            "having",
            "having",
            "generic",
        ),
        (
            "SELECT a FROM t LIMIT 10",
            "limit",
            "row_set_operator",
            "generic",
        ),
        (
            "SELECT a FROM t OFFSET 2",
            "offset",
            "row_set_operator",
            "generic",
        ),
        (
            "SELECT a FROM t FETCH FIRST 1 ROW ONLY",
            "fetch",
            "row_set_operator",
            "generic",
        ),
        (
            "SELECT a FROM t TABLESAMPLE SYSTEM (10)",
            "table_sample",
            "row_set_operator",
            "generic",
        ),
        (
            "SELECT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t QUALIFY rn = 1",
            "qualify",
            "qualify",
            "snowflake",
        ),
    ];
    for (sql, reason, clause, dialect) in cases {
        let columns = schema(
            "t",
            &[("a", "INTEGER"), ("b", "INTEGER"), ("name", "VARCHAR")],
            dialect,
        );
        let error = generate_classified_from_sql_with_assumptions(
            sql,
            dialect,
            &[columns],
            GenerationRowCounts::matching_only(5),
            42,
            &[],
        )
        .expect_err(sql);
        let ProtocolGenerationError::ResidualConditions { conditions, .. } = &error else {
            panic!("{sql}: unexpected error {error:?}");
        };
        assert!(
            conditions
                .iter()
                .any(|item| item.contains(&format!("reason={reason}"))
                    && item.contains(&format!("clause={clause}"))
                    && item.contains("layer=")),
            "{sql}: {conditions:?}"
        );
    }
}

#[test]
fn conditional_string_filters_require_explicit_declarations() {
    let schemas = [schema("t", &[("name", "VARCHAR")], "generic")];
    let sql = "SELECT name FROM t WHERE name = 'abc'";
    let error = generate_classified_from_sql_with_assumptions(
        sql,
        "generic",
        &schemas,
        GenerationRowCounts::matching_only(5),
        42,
        &[],
    )
    .expect_err("string comparisons require the binary collation assumption");
    assert!(
        matches!(
            error,
            ProtocolGenerationError::MissingComparisonAssumptions { .. }
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("binary_collation"));

    let generated = generate_classified_from_sql_with_assumptions(
        sql,
        "generic",
        &schemas,
        GenerationRowCounts::matching_only(5),
        42,
        &[ComparisonAssumption::BinaryCollation],
    )
    .expect("declared comparison assumptions allow exact generation");
    assert_eq!(generated.row_counts().matching(), 5);
}

#[test]
fn float_comparisons_require_all_assumptions() {
    let schemas = [schema("t", &[("value", "DOUBLE")], "generic")];
    let sql = "SELECT value FROM t WHERE value >= 1.0";
    let error = generate_classified_from_sql_with_assumptions(
        sql,
        "generic",
        &schemas,
        GenerationRowCounts::matching_only(5),
        42,
        &[ComparisonAssumption::NoNan],
    )
    .expect_err("missing signed zero equivalence must fail");
    assert!(error.to_string().contains("signed_zero_equivalent"));
    generate_classified_from_sql_with_assumptions(
        sql,
        "generic",
        &schemas,
        GenerationRowCounts::matching_only(5),
        42,
        &[
            ComparisonAssumption::NoNan,
            ComparisonAssumption::SignedZeroEquivalent,
        ],
    )
    .expect("complete declared assumptions allow generation");
}

#[test]
fn whole_project_mode_names_an_outcome_with_residual_conditions() {
    let sql = "CREATE VIEW good AS SELECT a FROM t WHERE a > 0;
               CREATE VIEW limited AS SELECT a FROM t WHERE a > 0 LIMIT 1;";
    let error = generate_classified_from_sql_with_assumptions(
        sql,
        "generic",
        &[schema("t", &[("a", "INTEGER")], "generic")],
        GenerationRowCounts::matching_only(5),
        42,
        &[],
    )
    .expect_err("one residual terminal outcome invalidates whole-project generation");
    let ProtocolGenerationError::TerminalOutcome { outcome, source } = error else {
        panic!("expected the failing terminal outcome");
    };
    assert!(outcome.contains("limited"));
    assert!(matches!(
        *source,
        ProtocolGenerationError::ResidualConditions { .. }
    ));
}

#[test]
fn intermediate_boundaries_reject_residual_target_conditions() {
    let sql = "CREATE VIEW stage AS SELECT a FROM t WHERE a > 0;
               CREATE VIEW limited AS SELECT a FROM stage LIMIT 1;";
    let boundary =
        GenerationBoundary::intermediate_relations(["stage"]).expect("valid intermediate boundary");
    let error = generate_classified_from_sql_at_boundary_with_assumptions(
        sql,
        "generic",
        &[
            schema("t", &[("a", "INTEGER")], "generic"),
            schema("stage", &[("a", "INTEGER")], "generic"),
        ],
        Some(&OutcomeSelector::Relation("limited".to_owned())),
        &boundary,
        SqlGenerationSettings::new(GenerationRowCounts::matching_only(5), 42, &[]),
    )
    .expect_err("boundary must reject residual target conditions");
    assert!(
        matches!(error, ProtocolGenerationError::ResidualConditions { .. }),
        "{error:?}"
    );
}

#[test]
fn implicit_where_join_equalities_coordinate_physical_keys() {
    let generated = generate_classified_from_sql_with_assumptions(
        "SELECT t.id FROM t, u WHERE t.id = u.id",
        "generic",
        &[
            schema("t", &[("id", "INTEGER")], "generic"),
            schema("u", &[("id", "INTEGER")], "generic"),
        ],
        GenerationRowCounts::matching_only(5),
        42,
        &[],
    )
    .expect("protocol-composed implicit join equalities must be honored");
    let left = generated
        .table("t")
        .expect("left relation")
        .get_ints("id")
        .expect("column")
        .expect("values");
    let right = generated
        .table("u")
        .expect("right relation")
        .get_ints("id")
        .expect("column")
        .expect("values");
    assert_eq!(left, right);
}

#[test]
fn missing_schema_columns_and_lossy_literals_are_residual() {
    for (sql, reason) in [
        ("SELECT a FROM t WHERE ghost = 1", "unknown_schema_column"),
        ("SELECT a FROM t WHERE a = 1.5", "lossy_coercion"),
    ] {
        let error = generate_classified_from_sql_with_assumptions(
            sql,
            "postgresql",
            &[schema("t", &[("a", "INTEGER")], "postgresql")],
            GenerationRowCounts::matching_only(4),
            42,
            &[],
        )
        .expect_err(sql);
        let ProtocolGenerationError::ResidualConditions { conditions, .. } = &error else {
            panic!("{sql}: unexpected {error:?}");
        };
        assert!(
            conditions
                .iter()
                .any(|condition| condition.contains(&format!("reason={reason}"))
                    && condition.contains("clause=where")),
            "{sql}: {conditions:?}"
        );
    }
}

#[test]
fn timezone_dependent_timestamp_filters_require_session_assumption() {
    let schemas = [schema("t", &[("tz", "TIMESTAMP WITH TIME ZONE")], "duckdb")];
    let sql = "SELECT tz FROM t WHERE tz >= TIMESTAMPTZ '2024-01-01 00:00:00+00'";
    let error = generate_classified_from_sql_with_assumptions(
        sql,
        "duckdb",
        &schemas,
        GenerationRowCounts::matching_only(4),
        42,
        &[],
    )
    .expect_err("session timezone must be explicitly declared");
    assert!(
        matches!(
            error,
            ProtocolGenerationError::MissingComparisonAssumptions { .. }
        ),
        "{error:?}"
    );
    assert!(error.to_string().contains("session_time_zone"));

    generate_classified_from_sql_with_assumptions(
        sql,
        "duckdb",
        &schemas,
        GenerationRowCounts::matching_only(4),
        42,
        &[ComparisonAssumption::SessionTimeZone],
    )
    .expect("attested timezone semantics must permit generation");
}
