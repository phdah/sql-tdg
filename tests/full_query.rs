use sql_tdg::{
    Column, ColumnType, Generator, Table, apply_conditions, parse_query, to_date, to_timestamp,
};

const ROWS: usize = 12;
const SEED: u64 = 42;

fn generate(query: &str, schema: Vec<Column>) -> Table {
    let parsed = parse_query(query).expect("query should parse");
    let mut table = Table::new(schema, ROWS).expect("table should be valid");

    apply_conditions(&parsed, &mut table).expect("query conditions should map to the schema");
    Generator::new()
        .generate(&mut table, SEED)
        .expect("generation should succeed");

    table
}

#[test]
fn full_query_generator_ints() {
    let mut single = generate(
        "SELECT col_a FROM t WHERE col_a = 10",
        vec![Column::new("col_a", ColumnType::Int)],
    );
    single.build_ints();

    assert_eq!(
        single
            .get_ints("col_a")
            .expect("integer column should be readable")
            .expect("integer column should be built"),
        vec![10; ROWS]
    );

    let mut multiple = generate(
        "SELECT col_a, col_b FROM t WHERE col_a > 5 OR col_a = 10 AND col_b = 5",
        vec![
            Column::new("col_a", ColumnType::Int),
            Column::new("col_b", ColumnType::Int),
        ],
    );
    multiple.build_ints();

    assert_eq!(
        multiple
            .get_ints("col_a")
            .expect("integer column should be readable")
            .expect("integer column should be built"),
        vec![10; ROWS]
    );
    assert_eq!(
        multiple
            .get_ints("col_b")
            .expect("integer column should be readable")
            .expect("integer column should be built"),
        vec![5; ROWS]
    );
}

#[test]
fn full_query_generator_bool() {
    let mut table = generate(
        "SELECT col_a FROM t WHERE col_a",
        vec![Column::new("col_a", ColumnType::Bool)],
    );
    table.build_bools();

    assert_eq!(
        table
            .get_bools("col_a")
            .expect("boolean column should be readable")
            .expect("boolean column should be built"),
        vec![true; ROWS]
    );
}

#[test]
fn full_query_generator_timestamp() {
    let cases = [
        (
            "SELECT col_a FROM t WHERE col_a = '2013-06-17'",
            to_date("2013-06-17").expect("test date should be valid"),
        ),
        (
            r#"SELECT col_a FROM t WHERE col_a = "2013-06-17T14:29:00Z""#,
            to_timestamp("2013-06-17T14:29:00Z").expect("test timestamp should be valid"),
        ),
    ];

    for (query, expected_seconds) in cases {
        let mut table = generate(query, vec![Column::new("col_a", ColumnType::Timestamp)]);
        table.build_timestamps();

        let values = table
            .get_timestamps("col_a")
            .expect("timestamp column should be readable")
            .expect("timestamp column should be built");

        assert_eq!(values.len(), ROWS);
        assert!(
            values
                .iter()
                .all(|value| value.timestamp() == i64::from(expected_seconds))
        );
    }
}

#[test]
fn full_query_seeded_generation_is_deterministic_and_satisfies_constraints() {
    let query = "SELECT col_a FROM t WHERE col_a > 3 AND col_a < 100 AND col_a != 10";
    let schema = vec![Column::new("col_a", ColumnType::Int)];

    let mut first = generate(query, schema.clone());
    let mut second = generate(query, schema);
    first.build_ints();
    second.build_ints();

    let first_values = first
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");
    let second_values = second
        .get_ints("col_a")
        .expect("integer column should be readable")
        .expect("integer column should be built");

    assert_eq!(first_values, second_values);
    assert_eq!(first_values.len(), ROWS);
    assert!(
        first_values
            .iter()
            .all(|value| *value > 3 && *value < 100 && *value != 10)
    );
}
