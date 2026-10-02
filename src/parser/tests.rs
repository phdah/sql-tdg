use super::{
    ConditionIR, ConditionOperator, JoinIR, JoinKind, ParserError, parse_query,
};

fn condition(left: &str, operator: ConditionOperator, right: &str) -> ConditionIR {
    ConditionIR::comparison(left.to_owned(), operator, right.to_owned())
}

#[test]
fn query_parsing() {
    let sql = r#"
        SELECT x,y
        FROM t
        LEFT JOIN u ON t.x = u.p
        JOIN u ON t.x = u.p AND t.x = u.r
        CROSS JOIN t ON t.a > t.l
        NATURAL JOIN t ON t.a > t.l
        WHERE x > 5 OR y = 10 AND t AND x = 10 AND t = "2025-06-19"
    "#;

    let query = parse_query(sql).expect("query should parse");

    assert_eq!(query.projection(), &["x".to_owned(), "y".to_owned()]);
    assert_eq!(query.source(), "t");
    assert_eq!(
        query.joins(),
        &[
            JoinIR {
                kind: JoinKind::Left,
                table: "u".to_owned(),
                conditions: vec![condition(
                    "t.x",
                    ConditionOperator::Equal,
                    "u.p"
                )],
            },
            JoinIR {
                kind: JoinKind::Inner,
                table: "u".to_owned(),
                conditions: vec![
                    condition("t.x", ConditionOperator::Equal, "u.p"),
                    condition("t.x", ConditionOperator::Equal, "u.r"),
                ],
            },
            JoinIR {
                kind: JoinKind::Cross,
                table: "t".to_owned(),
                conditions: vec![condition(
                    "t.a",
                    ConditionOperator::GreaterThan,
                    "t.l"
                )],
            },
            JoinIR {
                kind: JoinKind::NaturalInner,
                table: "t".to_owned(),
                conditions: vec![condition(
                    "t.a",
                    ConditionOperator::GreaterThan,
                    "t.l"
                )],
            },
        ]
    );
    assert_eq!(
        query.conditions(),
        &[
            condition("x", ConditionOperator::GreaterThan, "5"),
            condition("y", ConditionOperator::Equal, "10"),
            condition("t", ConditionOperator::Boolean, "true"),
            condition("x", ConditionOperator::Equal, "10"),
            condition(
                "t",
                ConditionOperator::Equal,
                r#""2025-06-19""#
            ),
        ]
    );
}

#[test]
fn qualify_and_functions_are_lowered() {
    let query = parse_query(
        "SELECT row_number(), account.id FROM account \
         WHERE account.enabled QUALIFY row_number() = 1",
    )
    .expect("query should parse");

    assert_eq!(
        query.projection(),
        &["row_number()".to_owned(), "account.id".to_owned()]
    );
    assert_eq!(
        query.conditions(),
        &[
            condition("account.enabled", ConditionOperator::Boolean, "true"),
            condition("row_number()", ConditionOperator::Equal, "1"),
        ]
    );
}

#[test]
fn all_supported_join_kinds_are_typed() {
    let cases = [
        ("JOIN", JoinKind::Inner),
        ("INNER JOIN", JoinKind::Inner),
        ("LEFT JOIN", JoinKind::Left),
        ("RIGHT JOIN", JoinKind::Right),
        ("FULL JOIN", JoinKind::Full),
        ("CROSS JOIN", JoinKind::Cross),
        ("NATURAL JOIN", JoinKind::NaturalInner),
        ("NATURAL LEFT JOIN", JoinKind::NaturalLeft),
        ("NATURAL RIGHT JOIN", JoinKind::NaturalRight),
        ("NATURAL FULL JOIN", JoinKind::NaturalFull),
    ];

    for (syntax, expected) in cases {
        let sql = format!("SELECT x FROM t {syntax} u ON t.x = u.x");
        let query = parse_query(&sql).expect("join should parse");
        assert_eq!(query.joins()[0].kind(), expected, "{syntax}");
    }
}

#[test]
fn unsupported_expression_returns_error() {
    let error = parse_query("SELECT x FROM t WHERE x + 1 > 2")
        .expect_err("arithmetic should not be lowered");

    assert!(matches!(error, ParserError::UnsupportedExpression(_)));
}

#[test]
fn unsupported_join_constraint_returns_error() {
    let error =
        parse_query("SELECT x FROM t JOIN u USING (x)").expect_err("USING should not be lowered");

    assert!(matches!(
        error,
        ParserError::UnsupportedJoinConstraint(_)
    ));
}

#[test]
fn unsupported_projection_returns_error() {
    let error = parse_query("SELECT * FROM t").expect_err("wildcard should not be lowered");

    assert!(matches!(error, ParserError::UnsupportedProjection(_)));
}

#[test]
fn unsupported_query_clause_returns_error() {
    let error =
        parse_query("SELECT x FROM t GROUP BY x").expect_err("GROUP BY should not be lowered");

    assert!(matches!(error, ParserError::UnsupportedQuery(_)));
}

#[test]
fn multiple_statements_return_error() {
    let error =
        parse_query("SELECT x FROM t; SELECT y FROM u").expect_err("one statement is required");

    assert_eq!(error, ParserError::ExpectedSingleStatement { count: 2 });
}

#[test]
fn block_comments_return_explicit_error() {
    let error =
        parse_query("SELECT x /* comment */ FROM t").expect_err("block comments are unsupported");

    assert_eq!(
        error,
        ParserError::UnsupportedSyntax("block comments".to_owned())
    );
}

#[test]
fn natural_identifier_is_not_treated_as_join_modifier() {
    let query =
        parse_query("SELECT natural FROM natural JOIN u ON natural.x = u.x").expect("valid query");

    assert_eq!(query.source(), "natural");
    assert_eq!(query.projection(), &["natural".to_owned()]);
    assert_eq!(query.joins()[0].kind(), JoinKind::Inner);
}
