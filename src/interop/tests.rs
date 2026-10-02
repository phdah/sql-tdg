use crate::{
    BoolConstraint, Column, ColumnType, Constraint, IntConstraint, Table, TimestampConstraint,
    parse_query,
};

use super::{InteropError, apply_conditions};

fn constraints<'a>(table: &'a Table, column: &str) -> &'a [Constraint] {
    table
        .schema()
        .iter()
        .find(|schema_column| schema_column.name() == column)
        .expect("test schema contains requested column")
        .constraints()
}

#[test]
fn integer_conditions_map_to_typed_constraints() {
    let query = parse_query(
        "SELECT col_a, col_b FROM t WHERE col_a > 5 OR col_a = 10 AND col_b = 5",
    )
    .expect("query should parse");
    let mut table = Table::new(
        vec![
            Column::new("col_a", ColumnType::Int),
            Column::new("col_b", ColumnType::Int),
        ],
        12,
    )
    .expect("table schema should be valid");

    apply_conditions(&query, &mut table).expect("conditions should map");

    assert_eq!(
        constraints(&table, "col_a"),
        &[
            Constraint::Int(IntConstraint::GreaterThan(5)),
            Constraint::Int(IntConstraint::Equal(10)),
        ]
    );
    assert_eq!(
        constraints(&table, "col_b"),
        &[Constraint::Int(IntConstraint::Equal(5))]
    );
}

#[test]
fn bare_boolean_condition_maps_to_true_constraint() {
    let query = parse_query("SELECT col_a FROM t WHERE col_a").expect("query should parse");
    let mut table = Table::new(vec![Column::new("col_a", ColumnType::Bool)], 12)
        .expect("table schema should be valid");

    apply_conditions(&query, &mut table).expect("condition should map");

    assert_eq!(
        constraints(&table, "col_a"),
        &[Constraint::Bool(BoolConstraint::IsTrue)]
    );
}

#[test]
fn timestamp_conditions_map_date_and_rfc3339_literals() {
    let cases = [
        (
            "SELECT col_a FROM t WHERE col_a = '2013-06-17'",
            1_371_427_200,
        ),
        (
            r#"SELECT col_a FROM t WHERE col_a = "2013-06-17T14:29:00Z""#,
            1_371_479_340,
        ),
    ];

    for (sql, expected) in cases {
        let query = parse_query(sql).expect("query should parse");
        let mut table = Table::new(vec![Column::new("col_a", ColumnType::Timestamp)], 12)
            .expect("table schema should be valid");

        apply_conditions(&query, &mut table).expect("condition should map");

        assert_eq!(
            constraints(&table, "col_a"),
            &[Constraint::Timestamp(TimestampConstraint::Equal(expected))]
        );
    }
}

#[test]
fn unknown_column_returns_error_without_mutating_schema() {
    let query =
        parse_query("SELECT col_a FROM t WHERE col_a = 10 AND missing = 5").expect("query parses");
    let mut table = Table::new(vec![Column::new("col_a", ColumnType::Int)], 12)
        .expect("table schema should be valid");

    let error = apply_conditions(&query, &mut table).expect_err("unknown column should fail");

    assert_eq!(
        error,
        InteropError::UnknownColumn {
            column: "missing".to_owned(),
        }
    );
    assert!(constraints(&table, "col_a").is_empty());
}

#[test]
fn unsupported_column_type_returns_error() {
    let query = parse_query("SELECT col_a FROM t WHERE col_a = 'value'").expect("query should parse");
    let mut table = Table::new(vec![Column::new("col_a", ColumnType::String)], 12)
        .expect("table schema should be valid");

    let error = apply_conditions(&query, &mut table).expect_err("string mapping is unsupported");

    assert_eq!(
        error,
        InteropError::UnsupportedColumnType {
            column: "col_a".to_owned(),
            column_type: ColumnType::String,
        }
    );
}

#[test]
fn unsupported_operator_returns_error() {
    let query = parse_query("SELECT flag FROM t WHERE flag > 1").expect("query should parse");
    let mut table = Table::new(vec![Column::new("flag", ColumnType::Bool)], 12)
        .expect("table schema should be valid");

    let error = apply_conditions(&query, &mut table).expect_err("operator should be rejected");

    assert_eq!(
        error,
        InteropError::UnsupportedOperator {
            column: "flag".to_owned(),
            column_type: ColumnType::Bool,
            operator: crate::ConditionOperator::GreaterThan,
        }
    );
}

#[test]
fn invalid_integer_returns_error() {
    let query =
        parse_query("SELECT col_a FROM t WHERE col_a = 2147483648").expect("query should parse");
    let mut table = Table::new(vec![Column::new("col_a", ColumnType::Int)], 12)
        .expect("table schema should be valid");

    let error = apply_conditions(&query, &mut table).expect_err("out-of-range int should fail");

    assert_eq!(
        error,
        InteropError::InvalidInteger {
            column: "col_a".to_owned(),
            value: "2147483648".to_owned(),
        }
    );
}

#[test]
fn invalid_timestamp_returns_error() {
    let query =
        parse_query("SELECT col_a FROM t WHERE col_a = 'not-a-date'").expect("query should parse");
    let mut table = Table::new(vec![Column::new("col_a", ColumnType::Timestamp)], 12)
        .expect("table schema should be valid");

    let error = apply_conditions(&query, &mut table).expect_err("invalid timestamp should fail");

    assert!(matches!(
        error,
        InteropError::InvalidTimestamp { column, value, .. }
            if column == "col_a" && value == "'not-a-date'"
    ));
}
