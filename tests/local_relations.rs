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
                SchemaColumn::from_sql_type(*name, data_type, "duckdb")
                    .expect("test datatype should normalize")
            })
            .collect(),
    )
    .expect("test relation schema should be valid")
}

fn generated_ints(data: &sql_tdg::GeneratedData, relation: &str, column: &str) -> Vec<i32> {
    data.table(relation)
        .expect("source relation should be generated")
        .get_ints(column)
        .expect("column should be readable")
        .expect("column should be built")
}

fn assert_all_rows_match_local_filter(
    sql: &str,
    schemas: &[RelationSchema],
    relation: &str,
    column: &str,
    range: std::ops::Range<i32>,
) {
    let generated = generate_from_sql(sql, "duckdb", schemas, ROWS, SEED)
        .expect("composed local-relation semantics should be generated");
    let values = generated_ints(&generated, relation, column);
    assert_eq!(values.len(), ROWS);
    assert!(
        values.iter().all(|value| range.contains(value)),
        "{values:?}"
    );
}

#[test]
fn cte_filter_is_applied_to_physical_source_rows() {
    assert_all_rows_match_local_filter(
        "WITH big AS (SELECT a FROM t WHERE a > 1000) SELECT a FROM big",
        &[schema("t", &[("a", "INTEGER")])],
        "t",
        "a",
        1001..i32::MAX,
    );
}

#[test]
fn derived_table_filter_is_applied_to_physical_source_rows() {
    assert_all_rows_match_local_filter(
        "SELECT a FROM (SELECT a FROM t WHERE a > 1000 AND a < 2000) AS big",
        &[schema("t", &[("a", "INTEGER")])],
        "t",
        "a",
        1001..2000,
    );
}

#[test]
fn chained_ctes_intersect_filters_without_dropping_either() {
    assert_all_rows_match_local_filter(
        "WITH first AS (SELECT a FROM t WHERE a > 1000),
              second AS (SELECT a FROM first WHERE a < 2000)
         SELECT a FROM second",
        &[schema("t", &[("a", "INTEGER")])],
        "t",
        "a",
        1001..2000,
    );
}

#[test]
fn cte_filters_propagate_across_upstream_views_and_intermediate_boundaries() {
    let sql = "
        CREATE VIEW stage AS WITH big AS (
            SELECT a FROM t WHERE a > 1000
        ) SELECT a FROM big;
        CREATE VIEW final AS SELECT a FROM stage WHERE a < 2000;
    ";
    let schemas = [
        schema("t", &[("a", "INTEGER")]),
        schema("stage", &[("a", "INTEGER")]),
    ];
    let generated = generate_from_sql(sql, "duckdb", &schemas, ROWS, SEED)
        .expect("physical generation must honor ancestor CTE filters");
    assert!(
        generated_ints(&generated, "t", "a")
            .iter()
            .all(|value| (1001..2000).contains(value))
    );

    let boundary =
        GenerationBoundary::intermediate_relations(["stage"]).expect("boundary should build");
    let generated = generate_from_sql_at_boundary(
        sql,
        "duckdb",
        &schemas,
        Some(&OutcomeSelector::Relation("final".to_owned())),
        &boundary,
        ROWS,
        SEED,
    )
    .expect("intermediate generation must use exact composed semantics");
    assert!(
        generated_ints(&generated, "stage", "a")
            .iter()
            .all(|value| (1001..2000).contains(value))
    );
}

#[test]
fn whole_project_intersects_local_relation_and_direct_filters() {
    let sql = "
        CREATE VIEW plain AS SELECT a FROM t WHERE a > 1500;
        CREATE VIEW nested AS WITH big AS (
            SELECT a FROM t WHERE a > 1000
        ) SELECT a FROM big WHERE a < 2000;
    ";
    let generated = generate_from_sql(
        sql,
        "duckdb",
        &[schema("t", &[("a", "INTEGER")])],
        ROWS,
        SEED,
    )
    .expect("all terminal outcomes should share their composed domains");
    assert!(
        generated_ints(&generated, "t", "a")
            .iter()
            .all(|value| (1501..2000).contains(value))
    );
}

#[test]
fn three_source_cte_chain_coordinates_inner_join_keys_in_duckdb() {
    let sql = "
        WITH paid_orders AS (
            SELECT id, customer_id, amount FROM orders
            WHERE amount >= 10 AND amount < 20
        ),
        enriched AS (
            SELECT paid_orders.customer_id, paid_orders.amount
            FROM paid_orders
            JOIN customers ON paid_orders.customer_id = customers.id
        ),
        line_items AS (
            SELECT enriched.customer_id, enriched.amount, items.product_id
            FROM enriched
            JOIN items ON enriched.customer_id = items.customer_id
            JOIN products ON items.product_id = products.id
        )
        SELECT customer_id, SUM(amount) AS revenue
        FROM line_items GROUP BY customer_id
    ";
    let generated = generate_from_sql(
        sql,
        "duckdb",
        &[
            schema(
                "orders",
                &[
                    ("id", "INTEGER"),
                    ("customer_id", "INTEGER"),
                    ("amount", "INTEGER"),
                ],
            ),
            schema("customers", &[("id", "INTEGER")]),
            schema(
                "items",
                &[("customer_id", "INTEGER"), ("product_id", "INTEGER")],
            ),
            schema("products", &[("id", "INTEGER")]),
        ],
        ROWS,
        SEED,
    )
    .expect("inner equalities within chained CTEs should coordinate physical keys");

    let customer_ids = generated_ints(&generated, "orders", "customer_id");
    let customer_keys = generated_ints(&generated, "customers", "id");
    let item_customers = generated_ints(&generated, "items", "customer_id");
    let product_ids = generated_ints(&generated, "items", "product_id");
    let product_keys = generated_ints(&generated, "products", "id");

    assert_eq!(customer_ids, customer_keys);
    assert_eq!(customer_ids, item_customers);
    assert_eq!(product_ids, product_keys);
    assert!(
        generated_ints(&generated, "orders", "amount")
            .iter()
            .all(|amount| (10..20).contains(amount))
    );
}

#[test]
fn unrepresentable_local_relation_filters_and_join_keys_fail_closed() {
    let cases = [
        (
            "WITH computed AS (SELECT a + 1 AS derived FROM t)
             SELECT derived FROM computed WHERE derived > 10",
            vec![schema("t", &[("a", "INTEGER")])],
        ),
        (
            "WITH grouped AS (SELECT a, SUM(b) AS total FROM t GROUP BY a)
             SELECT total FROM grouped WHERE total > 10",
            vec![schema("t", &[("a", "INTEGER"), ("b", "INTEGER")])],
        ),
        (
            "WITH ranked AS (SELECT a, ROW_NUMBER() OVER (ORDER BY a) AS rn FROM t)
             SELECT a FROM ranked WHERE rn = 1",
            vec![schema("t", &[("a", "INTEGER")])],
        ),
        (
            "WITH bucketed AS (SELECT CASE WHEN a > 10 THEN 1 ELSE 0 END AS kind FROM t)
             SELECT kind FROM bucketed WHERE kind = 1",
            vec![schema("t", &[("a", "INTEGER")])],
        ),
        (
            "WITH keyed AS (SELECT id + 1 AS shifted FROM t)
             SELECT shifted FROM keyed JOIN u ON keyed.shifted = u.id",
            vec![
                schema("t", &[("id", "INTEGER")]),
                schema("u", &[("id", "INTEGER")]),
            ],
        ),
    ];

    for (sql, schemas) in cases {
        let error = generate_from_sql(sql, "duckdb", &schemas, ROWS, SEED)
            .expect_err("local filters and joins without exact physical domains must fail");
        assert!(
            matches!(
                error,
                ProtocolGenerationError::ResidualConditions { .. }
                    | ProtocolGenerationError::UnsupportedSemantics { .. }
            ),
            "expected explicit protocol exactness failure for {sql}: {error:?}"
        );
    }
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

    let generated = generate_from_sql(sql, "duckdb", &schemas, ROWS, SEED)
        .expect("physical and produced relations remain supported");
    assert!(
        generated_ints(&generated, "t", "a")
            .iter()
            .all(|value| (10..=20).contains(value))
    );
}
