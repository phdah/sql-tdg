# sql-tdg

> SQL Test Data Generator

sql-tdg is a Rust library that generates Arrow-backed source data satisfying semantics resolved by
[SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol).

SQL Semantic Protocol is the sole SQL semantic boundary. sql-tdg does not parse SQL, derive
predicate intervals, resolve dialects, or compose lineage itself. It consumes protocol
`column_domains`, typed source schemas, terminal outcomes, and composed semantics and translates
them into deterministic generation plans.

## Quick start

The package exposes both a library API and an installable `sql-tdg` binary.

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
`generate_classified_from_bundle`. Matching scalar range values are sampled across their full
representable allowed intervals, while rejected range values are sampled from the representable
complement. Each rejected row deterministically selects one constrained scalar column with a safe
complement domain and samples every other column normally. The seed makes this random sampling
reproducible. Unbounded or otherwise insufficient domains fail explicitly when they cannot guarantee
the requested classification.

For supported inner equality relationships, multi-relation generation coordinates physical source
keys using protocol join, graph, schema, and lineage metadata. Matching rows satisfy every connected
equality relationship. Rejected relational rows deterministically break one safely isolatable
relationship while keeping scalar domains and the remaining relationships valid. Relationship
shapes that cannot guarantee those properties fail explicitly.

Unknown or empty value domains, unresolved composition, missing source schemas, ambiguous terminal
outcomes, and unsupported generation semantics are explicit errors. sql-tdg never reparses SQL as
a fallback.


Generated relations are backend-neutral Arrow data. Callers can consume the Arrow-backed tables
directly, convert a table to an Arrow `RecordBatch`, or export it as CSV or Parquet. Parquet is the
preferred lossless file format for the full supported Arrow type surface; CSV is an interoperability
format for flat data, including dbt seed workflows.

sql-tdg does not connect to, create, seed, or mutate user databases. Loading generated files into a
database or warehouse is owned by the caller. DuckDB is used only by this repository's end-to-end
test harness to prove that generated data can execute real SQL workloads.

The original Python proof of concept remains under `python_poc/` as frozen reference material and
is not part of the active implementation.


## CLI generation

The production CLI only generates backend-neutral relation files and reproducibility metadata. It
does not connect to, seed, execute against, or verify a database.

For raw SQL, declare the typed source schema explicitly. Repeat `--sql`, `--file`, and
`--schema` as needed:

```console
sql-tdg generate \
  --dialect duckdb \
  --sql 'SELECT amount FROM orders WHERE amount >= 10 AND amount < 20' \
  --schema 'orders:amount=INTEGER' \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format parquet \
  --output .sql-tdg/orders
```

Schema entries use `RELATION:COLUMN=SQL_TYPE`. SQL types are normalized by SQL Semantic Protocol
using the selected dialect. Parquet is the default and preferred lossless format. CSV is available
for flat interoperability workflows.

Use `--target <relation>` when a bundle has multiple named terminal outcomes, or
`--target-layer <layer-id>` for an anonymous terminal outcome. By default the CLI materializes
physical source relations. Repeat `--boundary <relation>` to materialize explicitly selected
intermediate relations instead.

For dbt, first produce normal dbt artifacts, including `catalog.json`, then point sql-tdg at the
project:

```console
dbt compile
dbt docs generate

sql-tdg generate \
  --dbt-project . \
  --target warehouse.analytics.customer_summary \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format parquet \
  --output .sql-tdg/customer-summary
```

You can also pass artifacts directly:

```console
sql-tdg generate \
  --dbt-manifest target/manifest.json \
  --dbt-catalog target/catalog.json \
  --target warehouse.analytics.customer_summary \
  --output .sql-tdg/customer-summary
```

The dbt adapter and dialect come from SQL Semantic Protocol. The CLI does not interpret dbt SQL or
artifact semantics itself.

Each successful run writes one deterministic relation file per generated relation plus
`metadata.sqltdg`. The metadata records the workload identity, selected target and boundary,
dialect, seed, generated relation row classifications, and the normalized SQL Semantic Protocol
snapshot needed to reproduce the generation inputs.

## Verification

Run the complete Rust verification suite with:

```console
make rust-checks
```
