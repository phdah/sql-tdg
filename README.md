# sql-tdg

> SQL Test Data Generator

sql-tdg is a Rust library that generates Arrow-backed source data satisfying semantics resolved by
[SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol).

SQL Semantic Protocol is the sole SQL semantic boundary. sql-tdg does not parse SQL, derive
predicate intervals, resolve dialects, or compose lineage itself. It consumes protocol
`column_domains`, typed source schemas, terminal outcomes, and composed semantics and translates
them into deterministic generation plans.

## Quick start

The project exposes a library API and has no CLI entry point.

```rust
use sql_tdg::{RelationSchema, SchemaColumn, generate_from_sql};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = RelationSchema::new(
        "orders",
        vec![
            SchemaColumn::from_sql_type("amount", "INTEGER", "postgresql")?,
            SchemaColumn::from_sql_type("created_at", "TIMESTAMPTZ", "postgresql")?,
        ],
    )?;

    let generated = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
        "postgresql",
        &[schema],
        100,
        42,
    )?;

    let values = generated
        .table("orders")
        .expect("orders is a physical source")
        .get_ints("amount")?
        .expect("generated column is built");

    assert!(values.iter().all(|value| (10..20).contains(value)));
    Ok(())
}
```

The SQL convenience API delegates analysis to SQL Semantic Protocol and then consumes the returned
bundle exactly like `generate_from_bundle`. For bundles with more than one terminal outcome,
callers must select an outcome explicitly rather than relying on an arbitrary default.

The Arrow-backed generator supports canonical protocol datatypes that it can represent losslessly,
including signed and unsigned integers, floating-point and decimal values, temporal types, strings,
binary values, nullable values, finite domains, arrays, maps, structs, and JSON-like storage. A
source datatype that sql-tdg cannot represent exactly returns `UnsupportedSourceType`; it is never
coerced to a weaker generation type.

For negative test data, `GenerationRowCounts` can be passed to `generate_classified_from_sql` or
`generate_classified_from_bundle`. Matching rows are emitted first. Each rejected row
deterministically selects one constrained scalar column with a safe complement domain, samples that
column outside its allowed protocol domain, and samples every other column normally. Unbounded or
otherwise insufficient domains fail explicitly when they cannot guarantee the requested
classification.

Unknown or empty value domains, unresolved composition, missing source schemas, ambiguous terminal
outcomes, and unsupported generation semantics are explicit errors. sql-tdg never reparses SQL as
a fallback.

The original Python proof of concept remains under `python_poc/` as frozen reference material and
is not part of the active implementation.

## Verification

Run the complete Rust verification suite with:

```console
make rust-checks
```
