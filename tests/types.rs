use sql_tdg::{
    BoolConstraint, Column, ColumnError, ColumnType, Constraint, IntConstraint, Interval,
    TimestampConstraint,
};

#[test]
fn column_type_names_match_reference_values() {
    assert_eq!(ColumnType::Int.as_str(), "int");
    assert_eq!(ColumnType::Timestamp.as_str(), "timestamp");
    assert_eq!(ColumnType::Bool.as_str(), "bool");
    assert_eq!(ColumnType::String.as_str(), "string");
}

#[test]
fn interval_preserves_inclusive_bounds() {
    let interval = Interval::new(-10, 10).expect("valid interval");

    assert_eq!(interval.min(), -10);
    assert_eq!(interval.max(), 10);
    assert!(interval.contains(-10));
    assert!(interval.contains(0));
    assert!(interval.contains(10));
    assert!(!interval.contains(11));
}

#[test]
fn interval_rejects_reversed_bounds() {
    let error = Interval::new(11, 10).expect_err("reversed bounds must fail");

    assert_eq!(error.min(), 11);
    assert_eq!(error.max(), 10);
}

#[test]
fn integer_constraints_cover_reference_operators() {
    let constraints = [
        IntConstraint::Equal(1),
        IntConstraint::NotEqual(2),
        IntConstraint::LessThan(3),
        IntConstraint::LessThanOrEqual(4),
        IntConstraint::GreaterThan(5),
        IntConstraint::GreaterThanOrEqual(6),
    ];

    assert_eq!(constraints.len(), 6);
}

#[test]
fn timestamp_constraints_cover_reference_operators() {
    let constraints = [
        TimestampConstraint::Equal(1),
        TimestampConstraint::NotEqual(2),
        TimestampConstraint::LessThan(3),
        TimestampConstraint::LessThanOrEqual(4),
        TimestampConstraint::GreaterThan(5),
        TimestampConstraint::GreaterThanOrEqual(6),
    ];

    assert_eq!(constraints.len(), 6);
}

#[test]
fn column_accepts_only_matching_constraints() {
    let mut column = Column::new("active", ColumnType::Bool);

    column
        .add_constraint(Constraint::Bool(BoolConstraint::IsTrue))
        .expect("boolean constraint should match boolean column");

    assert_eq!(column.name(), "active");
    assert_eq!(column.column_type(), ColumnType::Bool);
    assert_eq!(
        column.constraints(),
        &[Constraint::Bool(BoolConstraint::IsTrue)]
    );

    let error = column
        .add_constraint(Constraint::Int(IntConstraint::GreaterThan(10)))
        .expect_err("integer constraint must not be accepted by boolean column");

    assert_eq!(
        error,
        ColumnError::ConstraintTypeMismatch {
            column_type: ColumnType::Bool,
            constraint_type: ColumnType::Int,
        }
    );
}

#[test]
fn string_columns_cannot_receive_supported_constraints() {
    let mut column = Column::new("name", ColumnType::String);

    let error = column
        .add_constraint(Constraint::Bool(BoolConstraint::IsFalse))
        .expect_err("string constraints are not currently supported");

    assert_eq!(
        error,
        ColumnError::ConstraintTypeMismatch {
            column_type: ColumnType::String,
            constraint_type: ColumnType::Bool,
        }
    );
    assert!(column.constraints().is_empty());
}
