use sql_semantic_protocol::{DataType, DataTypeField, parse_data_type};
use sql_tdg::{RelationSchema, SchemaColumn, generate_from_sql};

const ROWS: usize = 32;
const SEED: u64 = 42;

fn column(name: &str, data_type: DataType) -> SchemaColumn {
    SchemaColumn::new(name, data_type).expect("type coverage schema column should be valid")
}

fn coverage_schema() -> RelationSchema {
    RelationSchema::new(
        "typed_source",
        vec![
            column("i64_value", DataType::SignedInteger { bits: Some(64) }),
            column("u64_value", DataType::UnsignedInteger { bits: Some(64) }),
            column(
                "decimal_value",
                DataType::Decimal {
                    precision: Some(18),
                    scale: Some(4),
                },
            ),
            column("float_value", DataType::FloatingPoint { bits: Some(64) }),
            column(
                "text_value",
                DataType::String {
                    length: Some(32),
                    fixed: false,
                },
            ),
            column(
                "binary_value",
                DataType::Binary {
                    length: Some(16),
                    fixed: false,
                },
            ),
            column("date_value", DataType::Date),
            column("time_value", DataType::Time { precision: Some(6) }),
            column(
                "timestamp_value",
                DataType::Timestamp { precision: Some(9) },
            ),
            column("uuid_value", DataType::Uuid),
            column("json_value", DataType::Json),
            column(
                "nullable_value",
                DataType::Nullable(Box::new(DataType::SignedInteger { bits: Some(64) })),
            ),
            column(
                "array_value",
                DataType::Array {
                    element: Some(Box::new(DataType::SignedInteger { bits: Some(64) })),
                    length: None,
                },
            ),
            column(
                "struct_value",
                DataType::Struct {
                    fields: vec![
                        DataTypeField::new(
                            Some("id".to_owned()),
                            DataType::SignedInteger { bits: Some(64) },
                        ),
                        DataTypeField::new(
                            Some("label".to_owned()),
                            DataType::String {
                                length: None,
                                fixed: false,
                            },
                        ),
                    ],
                },
            ),
            column(
                "map_value",
                DataType::Map {
                    key: Box::new(DataType::String {
                        length: None,
                        fixed: false,
                    }),
                    value: Box::new(DataType::SignedInteger { bits: Some(64) }),
                },
            ),
            column(
                "enum_value",
                parse_data_type("ENUM('ready', 'done')", "mysql")
                    .expect("enum datatype should normalize"),
            ),
            column(
                "set_value",
                parse_data_type("SET('red', 'blue')", "mysql")
                    .expect("set datatype should normalize"),
            ),
        ],
    )
    .expect("type coverage schema should be valid")
}

#[test]
fn all_supported_protocol_types_are_deterministic_and_preserve_arrow_types() {
    let schema = coverage_schema();
    let first = generate_from_sql(
        "SELECT * FROM typed_source",
        "generic",
        std::slice::from_ref(&schema),
        ROWS,
        SEED,
    )
    .expect("first generation should succeed");
    let second = generate_from_sql(
        "SELECT * FROM typed_source",
        "generic",
        &[schema],
        ROWS,
        SEED,
    )
    .expect("second generation should succeed");

    let first = first
        .table("typed_source")
        .expect("first generated source should exist");
    let second = second
        .table("typed_source")
        .expect("second generated source should exist");

    for field in first.arrow_schema().fields() {
        let name = field.name();
        let first_array = first
            .array(name)
            .expect("first array lookup should succeed")
            .expect("first array should be built");
        let second_array = second
            .array(name)
            .expect("second array lookup should succeed")
            .expect("second array should be built");

        assert_eq!(first_array.len(), ROWS, "{name}");
        assert_eq!(first_array.data_type(), field.data_type(), "{name}");
        assert_eq!(first_array.to_data(), second_array.to_data(), "{name}");
    }
}

#[test]
fn supported_type_boundaries_keep_expected_arrow_storage() {
    let generated = generate_from_sql(
        "SELECT * FROM typed_source",
        "generic",
        &[coverage_schema()],
        ROWS,
        SEED,
    )
    .expect("type coverage generation should succeed");
    let table = generated
        .table("typed_source")
        .expect("generated source should exist");

    assert_eq!(
        table
            .arrow_schema()
            .field_with_name("i64_value")
            .unwrap()
            .data_type(),
        &arrow_schema::DataType::Int64
    );
    assert_eq!(
        table
            .arrow_schema()
            .field_with_name("u64_value")
            .unwrap()
            .data_type(),
        &arrow_schema::DataType::UInt64
    );
    assert_eq!(
        table
            .arrow_schema()
            .field_with_name("decimal_value")
            .unwrap()
            .data_type(),
        &arrow_schema::DataType::Decimal128(18, 4)
    );
    assert_eq!(
        table
            .arrow_schema()
            .field_with_name("timestamp_value")
            .unwrap()
            .data_type(),
        &arrow_schema::DataType::Timestamp(arrow_schema::TimeUnit::Nanosecond, None)
    );
    assert_eq!(
        table
            .arrow_schema()
            .field_with_name("uuid_value")
            .unwrap()
            .data_type(),
        &arrow_schema::DataType::FixedSizeBinary(16)
    );
    assert!(matches!(
        table
            .arrow_schema()
            .field_with_name("array_value")
            .unwrap()
            .data_type(),
        arrow_schema::DataType::List(_)
    ));
    assert!(matches!(
        table
            .arrow_schema()
            .field_with_name("struct_value")
            .unwrap()
            .data_type(),
        arrow_schema::DataType::Struct(_)
    ));
    assert!(matches!(
        table
            .arrow_schema()
            .field_with_name("map_value")
            .unwrap()
            .data_type(),
        arrow_schema::DataType::Map(_, false)
    ));
}
