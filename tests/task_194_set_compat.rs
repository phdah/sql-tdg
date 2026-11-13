//! TASK-194 exact protocol commit consumer guard: accepted SQL set parsers
//! must not let a scalar-only generator advertise unproved terminal row bags.

use sql_tdg::{
    generate_classified_from_sql_with_assumptions, GenerationRowCounts, RelationSchema,
    SchemaColumn,
};

fn schema(name: &str) -> RelationSchema {
    RelationSchema::new(
        name,
        ["id", "amount"]
            .into_iter()
            .map(|col| SchemaColumn::from_sql_type(col, "INTEGER", "generic").expect("integer schema"))
            .collect(),
    )
    .expect("source schema")
}

#[test]
fn all_supported_set_parsers_keep_unattested_generation_fail_closed() {
    let sources = [schema("orders"), schema("customers")];
    for sql in [
        "SELECT id FROM orders UNION ALL SELECT id FROM customers",
        "SELECT id FROM orders INTERSECT ALL SELECT id FROM customers",
        "SELECT id FROM orders EXCEPT ALL SELECT id FROM customers",
        "SELECT id, amount FROM orders UNION BY NAME SELECT amount, id FROM customers",
        "SELECT id FROM orders UNION SELECT id, amount FROM customers",
    ] {
        let result = generate_classified_from_sql_with_assumptions(
            sql,
            "generic",
            &sources,
            GenerationRowCounts::matching_only(5),
            42,
            &[],
        );
        assert!(
            result.is_err(),
            "{sql}: parser/output acceptance must not imply a certified generator source-bag goal"
        );
    }
}
