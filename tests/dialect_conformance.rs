use sql_tdg::{GenerationRowCounts, RelationSchema, SchemaColumn, generate_classified_from_sql, generate_from_sql};

// These are protocol dialect families, not execution-engine certifications.
// Keep this list aligned with docs/dialect-conformance.md and the protocol's
// publicly exposed --dialect values.
const DIALECTS: &[&str] = &[
    "ansi",
    "bigquery",
    "clickhouse",
    "databricks",
    "duckdb",
    "generic",
    "hive",
    "mssql",
    "mysql",
    "postgresql",
    "redshift",
    "snowflake",
    "sqlite",
];

fn schema(dialect: &str, relation: &str) -> RelationSchema {
    RelationSchema::new(
        relation,
        vec![
            SchemaColumn::from_sql_type("id", "INTEGER", dialect)
                .expect("fixture identifier type"),
            SchemaColumn::from_sql_type("value", "INTEGER", dialect)
                .expect("fixture value type"),
        ],
    )
    .expect("fixture relation")
}

#[test]
fn supported_dialects_generate_exact_scalar_filter_witnesses() {
    let sql = "SELECT value FROM t WHERE value >= 10 AND value < 20";

    for dialect in DIALECTS {
        let inputs = [schema(dialect, "t")];
        let first = generate_from_sql(sql, dialect, &inputs, 8, 42)
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        let second = generate_from_sql(sql, dialect, &inputs, 8, 42)
            .unwrap_or_else(|error| panic!("{dialect}: repeat failed: {error}"));
        let values = first
            .table("t")
            .expect("physical source")
            .get_ints("value")
            .expect("typed column")
            .expect("integer values");
        assert_eq!(values.len(), 8, "{dialect}");
        assert!(values.iter().all(|value| (10..20).contains(value)), "{dialect}");
        assert_eq!(
            values,
            second
                .table("t")
                .expect("repeat source")
                .get_ints("value")
                .expect("repeat column")
                .expect("repeat values"),
            "{dialect}: seed must produce identical values"
        );
    }
}

#[test]
fn supported_dialects_generate_inner_join_witnesses() {
    let sql = "SELECT t.id FROM t INNER JOIN u ON t.id = u.id WHERE t.id >= 10";

    for dialect in DIALECTS {
        let generated = generate_from_sql(
            sql,
            dialect,
            &[schema(dialect, "t"), schema(dialect, "u")],
            8,
            42,
        )
        .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        let left = generated
            .table("t")
            .expect("join left source")
            .get_ints("id")
            .expect("join left column")
            .expect("join left values");
        let right = generated
            .table("u")
            .expect("join right source")
            .get_ints("id")
            .expect("join right column")
            .expect("join right values");
        assert_eq!(left, right, "{dialect}: join keys must match");
        assert!(left.iter().all(|value| *value >= 10), "{dialect}");
    }
}

#[test]
fn supported_dialects_preserve_filter_domains_through_ctes() {
    let sql = "WITH filtered AS (SELECT value FROM t WHERE value >= 10) \
               SELECT value FROM filtered WHERE value < 20";

    for dialect in DIALECTS {
        let generated = generate_from_sql(sql, dialect, &[schema(dialect, "t")], 8, 42)
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        let values = generated
            .table("t")
            .expect("CTE physical source")
            .get_ints("value")
            .expect("CTE column")
            .expect("CTE values");
        assert_eq!(values.len(), 8, "{dialect}");
        assert!(values.iter().all(|value| (10..20).contains(value)), "{dialect}");
    }
}

#[test]
fn supported_dialects_allow_projection_only_aggregates() {
    // This tests source-row generation, not a claim about specific COUNT results.
    let sql = "SELECT value, COUNT(*) AS total FROM t WHERE value >= 10 GROUP BY value";

    for dialect in DIALECTS {
        let generated = generate_from_sql(sql, dialect, &[schema(dialect, "t")], 8, 42)
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        let values = generated
            .table("t")
            .expect("aggregate physical source")
            .get_ints("value")
            .expect("aggregate source column")
            .expect("aggregate source values");
        assert!(values.iter().all(|value| *value >= 10), "{dialect}");
    }
}

#[test]
fn having_outcomes_fail_closed_in_every_dialect() {
    let sql = "SELECT value FROM t GROUP BY value HAVING COUNT(*) > 1";

    for dialect in DIALECTS {
        let error = generate_classified_from_sql(
            sql,
            dialect,
            &[schema(dialect, "t")],
            GenerationRowCounts::matching_only(8),
            42,
        )
        .expect_err("unsupported HAVING membership must not emit matching rows");
        assert!(
            error.to_string().contains("having"),
            "{dialect}: unexpected error: {error}"
        );
    }
}

#[test]
fn dialect_specific_identifier_forms_use_the_protocol_parser() {
    for (dialect, sql) in [
        ("bigquery", "SELECT `value` FROM `t` WHERE `value` >= 10"),
        ("mysql", "SELECT `value` FROM `t` WHERE `value` >= 10"),
        ("mssql", "SELECT [value] FROM [t] WHERE [value] >= 10"),
        ("postgresql", "SELECT \"value\" FROM \"t\" WHERE \"value\" >= 10"),
    ] {
        let generated = generate_from_sql(sql, dialect, &[schema(dialect, "t")], 8, 42)
            .unwrap_or_else(|error| panic!("{dialect}: {error}"));
        assert!(generated.table("t").is_some(), "{dialect}: expected physical source");
    }
}
