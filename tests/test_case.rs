use sql_tdg::{
    GeneratedRelation, GenerationBoundary, ProtocolSnapshot, QueryResult, ResultColumn,
    ResultOrdering, TestCase, TestCaseError, TestCaseMetadata, TestTarget, VerificationError,
    WorkloadIdentity,
};

fn metadata(boundary: GenerationBoundary) -> TestCaseMetadata {
    TestCaseMetadata::new(
        WorkloadIdentity::raw_sql("orders regression", "queries/orders.sql")
            .expect("test workload identity should be valid"),
        TestTarget::relation("analytics.orders").expect("test target identity should be valid"),
        boundary,
        "duckdb",
        42,
        ProtocolSnapshot::new("{\n  \"protocol_version\": \"1.0\"\n}")
            .expect("test protocol snapshot should be valid"),
        vec![
            GeneratedRelation::new("stage.orders", 8, 2).expect("test relation should be valid"),
            GeneratedRelation::new("raw.orders", 8, 2).expect("test relation should be valid"),
        ],
    )
    .expect("test metadata should be valid")
}

fn result(rows: &[&[&str]]) -> QueryResult {
    QueryResult::new(
        vec![
            ResultColumn::new("order_id", "BIGINT").expect("test result column should be valid"),
            ResultColumn::new("status", "VARCHAR").expect("test result column should be valid"),
        ],
        rows.iter()
            .map(|row| row.iter().map(|value| (*value).to_owned()).collect())
            .collect(),
    )
    .expect("test query result should be valid")
}

#[test]
fn metadata_round_trip_is_canonical_and_reproducible() {
    let metadata = metadata(
        GenerationBoundary::intermediate_relations(["stage.orders"])
            .expect("test boundary should be valid"),
    );

    let serialized = metadata.serialize();
    let decoded =
        TestCaseMetadata::deserialize(&serialized).expect("serialized metadata should round-trip");

    assert_eq!(decoded, metadata);
    assert_eq!(decoded.serialize(), serialized);
    assert_eq!(decoded.seed(), 42);
    assert_eq!(decoded.dialect(), "duckdb");
    assert_eq!(
        decoded.protocol().document(),
        "{\n  \"protocol_version\": \"1.0\"\n}"
    );
    assert_eq!(decoded.relations()[0].relation(), "raw.orders");
    assert_eq!(decoded.relations()[1].relation(), "stage.orders");
    assert_eq!(decoded.relations()[0].rows().matching(), 8);
    assert_eq!(decoded.relations()[0].rows().rejected(), 2);
}

#[test]
fn dbt_workload_uses_the_same_metadata_contract() {
    let metadata = TestCaseMetadata::new(
        WorkloadIdentity::dbt_project("warehouse", "model:final_orders")
            .expect("dbt workload identity should be valid"),
        TestTarget::relation("analytics.final_orders")
            .expect("test target identity should be valid"),
        GenerationBoundary::physical_sources(),
        "duckdb",
        7,
        ProtocolSnapshot::new("{\"protocol_version\":\"1.0\"}")
            .expect("test protocol snapshot should be valid"),
        vec![GeneratedRelation::new("raw.orders", 10, 3).expect("test relation should be valid")],
    )
    .expect("dbt metadata should be valid");

    let decoded = TestCaseMetadata::deserialize(&metadata.serialize())
        .expect("dbt metadata should round-trip");

    assert_eq!(decoded, metadata);
}

#[test]
fn intermediate_boundary_must_be_materialized() {
    let error = TestCaseMetadata::new(
        WorkloadIdentity::raw_sql("orders", "queries/orders.sql")
            .expect("test workload identity should be valid"),
        TestTarget::relation("analytics.orders").expect("test target identity should be valid"),
        GenerationBoundary::intermediate_relations(["stage.missing"])
            .expect("boundary itself should be valid"),
        "duckdb",
        42,
        ProtocolSnapshot::new("{\"protocol_version\":\"1.0\"}")
            .expect("test protocol snapshot should be valid"),
        vec![GeneratedRelation::new("raw.orders", 8, 2).expect("test relation should be valid")],
    )
    .expect_err("missing intermediate materialization must be rejected");

    assert_eq!(
        error,
        TestCaseError::MissingBoundaryRelation {
            relation: "stage.missing".to_owned(),
        }
    );
}

#[test]
fn verification_requires_an_explicit_approval() {
    let current = result(&[&["1", "paid"]]);
    let test_case = TestCase::new(metadata(GenerationBoundary::physical_sources()));

    assert_eq!(
        test_case.verify_result(&current),
        Err(VerificationError::MissingApproval)
    );
    assert!(test_case.approved_result().is_none());
}

#[test]
fn unordered_approval_compares_complete_rows_as_a_multiset() {
    let approved = result(&[&["1", "paid"], &["2", "pending"]]);
    let same_rows_different_order = result(&[&["2", "pending"], &["1", "paid"]]);
    let changed = result(&[&["1", "paid"], &["3", "pending"]]);
    let mut test_case = TestCase::new(metadata(GenerationBoundary::physical_sources()));

    test_case.approve_result(approved.clone(), ResultOrdering::Unordered);

    assert_eq!(test_case.verify_result(&same_rows_different_order), Ok(()));
    assert_eq!(
        test_case.verify_result(&changed),
        Err(VerificationError::ResultMismatch)
    );
    assert_eq!(
        test_case
            .approved_result()
            .expect("approval should remain present")
            .result(),
        &approved
    );
}

#[test]
fn ordered_approval_preserves_observable_row_order() {
    let approved = result(&[&["1", "paid"], &["2", "pending"]]);
    let reordered = result(&[&["2", "pending"], &["1", "paid"]]);
    let mut test_case = TestCase::new(metadata(GenerationBoundary::physical_sources()));

    test_case.approve_result(approved, ResultOrdering::Ordered);

    assert_eq!(
        test_case.verify_result(&reordered),
        Err(VerificationError::ResultMismatch)
    );
}

#[test]
fn query_result_rejects_rows_with_the_wrong_width() {
    let error = QueryResult::new(
        vec![ResultColumn::new("order_id", "BIGINT").expect("test result column should be valid")],
        vec![vec!["1".to_owned(), "extra".to_owned()]],
    )
    .expect_err("row width mismatch must be rejected");

    assert_eq!(
        error,
        TestCaseError::InvalidResultRowWidth {
            row_index: 0,
            expected: 1,
            actual: 2,
        }
    );
}
