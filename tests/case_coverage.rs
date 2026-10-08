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
fn branch_eliminated_by_filter_is_reported_unreachable() {
    let generated = generate_from_sql(
        "SELECT CASE WHEN a < 3 THEN 1 ELSE 2 END AS bucket FROM t WHERE a > 5",
        "generic",
        &[schema()],
        10,
        42,
    )
    .expect("unreachable CASE branch does not invalidate row membership");
    assert!(generated.case_coverage().iter().any(|finding| {
        finding.status() == sql_tdg::CaseCoverageStatus::Unreachable
            && finding.detail().contains("composed query domains")
    }));
}

#[test]
fn unreachable_case_branch_is_reported() {
    let generated = generate_from_sql(
        "SELECT CASE WHEN a = 1 THEN 1 WHEN a = 1 THEN 2 ELSE 3 END AS bucket FROM t",
        "generic",
        &[schema()],
        10,
        42,
    )
    .expect("unreachable CASE branch is reported");
    assert!(
        generated
            .case_coverage()
            .iter()
            .any(|finding| { finding.status() == sql_tdg::CaseCoverageStatus::Unreachable })
    );
}

#[test]
fn aggregate_case_is_reported_as_not_coverable() {
    let generated = generate_from_sql(
        "SELECT CASE WHEN SUM(a) > 10 THEN 1 ELSE 2 END AS bucket FROM t",
        "generic",
        &[schema()],
        10,
        42,
    )
    .expect("unknown branch source conditions are reported");
    assert!(
        generated
            .case_coverage()
            .iter()
            .any(|finding| { finding.status() == sql_tdg::CaseCoverageStatus::Unknown })
    );
}

#[test]
fn insufficient_rows_are_reported_instead_of_claiming_coverage() {
    let generated = generate_from_sql(
        "SELECT CASE WHEN a = 1 THEN 1 WHEN a = 2 THEN 2 ELSE 3 END AS bucket FROM t",
        "generic",
        &[schema()],
        2,
        42,
    )
    .expect("source rows still satisfy the query");
    assert!(
        generated
            .case_coverage()
            .iter()
            .any(|finding| { finding.status() == sql_tdg::CaseCoverageStatus::InsufficientRows })
    );
}

#[test]
fn string_case_branches_require_attested_comparison_semantics() {
    use sql_semantic_protocol::ComparisonAssumption;
    use sql_tdg::{
        GenerationRowCounts, generate_classified_from_sql_with_assumptions,
    };

    let schema = RelationSchema::new(
        "t",
        vec![
            SchemaColumn::from_sql_type("name", "VARCHAR", "generic")
                .expect("string fixture type"),
        ],
    )
    .expect("fixture relation");
    let sql = "SELECT CASE WHEN name = 'alice' THEN 1 ELSE 2 END AS kind FROM t";
    let rows = GenerationRowCounts::matching_only(8);
    let err = generate_classified_from_sql_with_assumptions(
        sql, "generic", &[schema.clone()], rows, 42, &[],
    )
    .expect_err("without binary collation, CASE coverage must not claim witnesses");
    assert!(matches!(err, ProtocolGenerationError::CaseCoverage { .. }));
    assert!(err.to_string().contains("binary_collation"));

    let generated = generate_classified_from_sql_with_assumptions(
        sql, "generic", &[schema], rows, 42,
        &[ComparisonAssumption::BinaryCollation],
    )
    .expect("attested comparison setting permits coverage");
    assert_eq!(generated.case_coverage().len(), 2);
    assert!(generated.case_coverage().iter().all(|finding|
        finding.status() == sql_tdg::CaseCoverageStatus::Covered
    ));
}
