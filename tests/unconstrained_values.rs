use std::collections::BTreeSet;

use arrow_array::cast::AsArray;
use arrow_array::types::{
    Date32Type, Decimal128Type, Float64Type, Int8Type, Int64Type, TimestampMicrosecondType,
    UInt32Type,
};
use arrow_array::{Array, BooleanArray};
use sql_tdg::{GeneratedData, RelationSchema, SchemaColumn, Table, generate_from_sql};

const ROWS: usize = 200;
const SEED: u64 = 42;

/// 2020-01-01 and 2025-12-31 as days since the Unix epoch.
const FIRST_DAY: i32 = 18_262;
const LAST_DAY: i32 = 20_453;

fn schema() -> RelationSchema {
    let columns = [
        ("big_id", "BIGINT"),
        ("tiny", "TINYINT"),
        ("unsigned_count", "INTEGER UNSIGNED"),
        ("price", "DECIMAL(10,2)"),
        ("ratio", "DOUBLE"),
        ("label", "VARCHAR"),
        ("short_label", "VARCHAR(3)"),
        ("flag", "BOOLEAN"),
        ("day", "DATE"),
        ("created_at", "TIMESTAMP"),
        ("filtered", "BIGINT"),
    ];
    RelationSchema::new(
        "facts",
        columns
            .iter()
            .map(|(name, data_type)| {
                SchemaColumn::from_sql_type(*name, data_type, "generic")
                    .expect("test datatype should normalize")
            })
            .collect(),
    )
    .expect("test relation schema should be valid")
}

fn generate() -> GeneratedData {
    generate_from_sql(
        "SELECT * FROM facts WHERE filtered >= 5000000000",
        "generic",
        &[schema()],
        ROWS,
        SEED,
    )
    .expect("generation should succeed")
}

fn facts(data: &GeneratedData) -> &Table {
    data.table("facts").expect("facts should be generated")
}

fn array<'a>(table: &'a Table, column: &str) -> &'a dyn Array {
    table
        .array(column)
        .expect("array lookup should succeed")
        .expect("array should be built")
}

fn assert_varied<T: Ord>(values: impl IntoIterator<Item = T>, column: &str) {
    let distinct = values.into_iter().collect::<BTreeSet<_>>().len();
    assert!(
        distinct >= 50,
        "{column} has only {distinct} distinct values"
    );
}

#[test]
fn unconstrained_integers_are_moderate_and_varied() {
    let data = generate();
    let table = facts(&data);

    let big = array(table, "big_id")
        .as_primitive::<Int64Type>()
        .values()
        .to_vec();
    assert!(big.iter().all(|value| (1..=1_000).contains(value)));
    assert_varied(big, "big_id");

    let tiny = array(table, "tiny")
        .as_primitive::<Int8Type>()
        .values()
        .to_vec();
    assert!(tiny.iter().all(|value| (1..=127).contains(value)));
    assert_varied(tiny, "tiny");

    let unsigned = array(table, "unsigned_count")
        .as_primitive::<UInt32Type>()
        .values()
        .to_vec();
    assert!(unsigned.iter().all(|value| (1..=1_000).contains(value)));
    assert_varied(unsigned, "unsigned_count");
}

#[test]
fn unconstrained_decimals_and_floats_are_moderate_and_varied() {
    let data = generate();
    let table = facts(&data);

    let prices = array(table, "price")
        .as_primitive::<Decimal128Type>()
        .values()
        .to_vec();
    assert!(prices.iter().all(|value| (0..=100_000).contains(value)));
    assert_varied(prices, "price");

    let ratios = array(table, "ratio")
        .as_primitive::<Float64Type>()
        .values()
        .to_vec();
    assert!(ratios.iter().all(|value| (0.0..=1_000.0).contains(value)));
    assert_varied(ratios.iter().map(|value| value.to_bits()), "ratio");
}

#[test]
fn unconstrained_strings_and_booleans_are_varied() {
    let data = generate();
    let table = facts(&data);

    let labels = array(table, "label").as_string::<i32>();
    let labels = (0..labels.len())
        .map(|row| labels.value(row).to_owned())
        .collect::<Vec<_>>();
    assert!(labels.iter().all(|value| !value.is_empty()));
    assert_varied(labels, "label");

    let short = array(table, "short_label").as_string::<i32>();
    assert!((0..short.len()).all(|row| short.value(row).chars().count() <= 3));

    let flags = array(table, "flag")
        .as_any()
        .downcast_ref::<BooleanArray>()
        .expect("flag should be boolean");
    let flags = (0..flags.len())
        .map(|row| flags.value(row))
        .collect::<BTreeSet<_>>();
    assert_eq!(flags.len(), 2);
}

#[test]
fn unconstrained_temporal_values_are_recent_and_varied() {
    let data = generate();
    let table = facts(&data);

    let days = array(table, "day")
        .as_primitive::<Date32Type>()
        .values()
        .to_vec();
    assert!(
        days.iter()
            .all(|value| (FIRST_DAY..=LAST_DAY).contains(value))
    );
    assert_varied(days, "day");

    let first_micros = i64::from(FIRST_DAY) * 86_400_000_000;
    let last_micros = (i64::from(LAST_DAY) + 1) * 86_400_000_000 - 1;
    let created = array(table, "created_at")
        .as_primitive::<TimestampMicrosecondType>()
        .values()
        .to_vec();
    assert!(
        created
            .iter()
            .all(|value| (first_micros..=last_micros).contains(value))
    );
    assert_varied(created, "created_at");
}

#[test]
fn constrained_columns_keep_their_protocol_domain() {
    let data = generate();
    let table = facts(&data);

    let filtered = array(table, "filtered")
        .as_primitive::<Int64Type>()
        .values()
        .to_vec();
    assert!(filtered.iter().all(|value| *value >= 5_000_000_000));
}

#[test]
fn unconstrained_values_are_deterministic() {
    let first = generate();
    let second = generate();
    let (first, second) = (facts(&first), facts(&second));

    for field in first.arrow_schema().fields() {
        assert_eq!(
            array(first, field.name()).to_data(),
            array(second, field.name()).to_data(),
            "{}",
            field.name()
        );
    }
}
