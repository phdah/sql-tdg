use sql_tdg::{ProtocolGenerationError, RelationSchema, SchemaColumn, generate_from_sql};

fn schema() -> RelationSchema {
    RelationSchema::new(
        "t",
        vec![SchemaColumn::from_sql_type("a", "INTEGER", "generic").expect("integer fixture type")],
    )
    .expect("fixture relation")
}

fn values(sql: &str, rows: usize) -> Result<Vec<i32>, ProtocolGenerationError> {
    let data = generate_from_sql(sql, "generic", &[schema()], rows, 42)?;
    let values = data
        .table("t")
        .expect("physical source table")
        .get_ints("a")
        .expect("integer column")
        .expect("present integer column");
    Ok(values)
}

#[test]
fn searched_case_reaches_every_branch_and_else_within_filter() {
    let rows = values(
        "SELECT CASE WHEN a < 5 THEN 1 WHEN a < 10 THEN 2 ELSE 3 END AS bucket
         FROM t WHERE a >= 1 AND a <= 20",
        24,
    )
    .expect("supported CASE branches can be witnessed");
    assert!(rows.iter().all(|a| (1..=20).contains(a)));
    assert!(rows.iter().any(|a| *a < 5));
    assert!(rows.iter().any(|a| (5..10).contains(a)));
    assert!(rows.iter().any(|a| *a >= 10));
}

#[test]
fn simple_case_reaches_when_values_and_else() {
    let rows = values(
        "SELECT CASE a WHEN 1 THEN 10 WHEN 2 THEN 20 ELSE 30 END AS code
         FROM t WHERE a BETWEEN 1 AND 3",
        10,
    )
    .expect("simple CASE branches can be witnessed");
    assert!(rows.contains(&1));
    assert!(rows.contains(&2));
    assert!(rows.contains(&3));
}

#[test]
fn cte_output_case_is_covered() {
    let rows = values(
        "WITH classified AS
          (SELECT a, CASE WHEN a < 5 THEN 1 ELSE 2 END AS bucket FROM t)
         SELECT bucket FROM classified WHERE a BETWEEN 1 AND 9",
        10,
    )
    .expect("CASE semantics in CTE outputs are composed");
    assert!(rows.iter().all(|a| (1..=9).contains(a)));
    assert!(rows.iter().any(|a| *a < 5));
    assert!(rows.iter().any(|a| *a >= 5));
}

#[test]
fn branch_eliminated_by_query_filter_fails_explicitly() {
    let error = values(
        "SELECT CASE WHEN a < 3 THEN 1 ELSE 2 END AS bucket FROM t WHERE a > 5",
        10,
    )
    .expect_err("filtered CASE branch cannot be claimed covered");
    assert!(matches!(
        error,
        ProtocolGenerationError::CaseCoverage { .. }
    ));
    assert!(error.to_string().contains("unreachable under"));
}

#[test]
fn unreachable_case_branch_is_not_silently_skipped() {
    let error = values(
        "SELECT CASE WHEN a = 1 THEN 1 WHEN a = 1 THEN 2 ELSE 3 END AS bucket FROM t",
        10,
    )
    .expect_err("repeated WHEN is unreachable");
    assert!(matches!(
        error,
        ProtocolGenerationError::CaseCoverage { .. }
    ));
    assert!(error.to_string().contains("unreachable"));
}

#[test]
fn aggregate_case_is_reported_as_not_coverable() {
    let error = values(
        "SELECT CASE WHEN SUM(a) > 10 THEN 1 ELSE 2 END AS bucket FROM t",
        10,
    )
    .expect_err("aggregate-derived branch domains are unknown");
    assert!(matches!(
        error,
        ProtocolGenerationError::CaseCoverage { .. }
    ));
    assert!(error.to_string().contains("not coverable"));
}

#[test]
fn insufficient_rows_cannot_claim_case_coverage() {
    let error = values(
        "SELECT CASE WHEN a = 1 THEN 1 WHEN a = 2 THEN 2 ELSE 3 END AS bucket FROM t",
        2,
    )
    .expect_err("three branches need three matching rows");
    assert!(matches!(
        error,
        ProtocolGenerationError::CaseCoverage { .. }
    ));
    assert!(error.to_string().contains("at least 3 matching rows"));
}
