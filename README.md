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
bundle exactly like `generate_from_bundle`. Without an outcome selector, a bundle with more than one
terminal outcome generates one shared set of physical source tables: every row of a source
satisfies the supported conditions of every terminal outcome that reads it. Columns and
relationship keys constrained by several outcomes use values that satisfy all of their domains,
and conflicting outcomes fail with `ConflictingOutcomes` naming them. Rejected rows and
intermediate boundaries require selecting one outcome.

The Arrow-backed generator supports canonical protocol datatypes that it can represent losslessly,
including signed and unsigned integers, floating-point and decimal values, temporal types, strings,
binary values, nullable values, finite domains, arrays, maps, structs, and JSON-like storage. A
source datatype that sql-tdg cannot represent exactly returns `UnsupportedSourceType`; it is never
coerced to a weaker generation type.

For negative test data, `GenerationRowCounts` can be passed to `generate_classified_from_sql` or
`generate_classified_from_bundle`. Columns without any protocol domain are sampled from varied,
moderate values: integers 1..=1000 (capped by the type), decimals and floats 0..=1000, dates and
timestamps in 2020-2025, numbered strings, and NULL for one in ten values of nullable columns.
Matching scalar range values are sampled across their full
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

## Exactness and comparison assumptions

Generation is allowed only when SQL Semantic Protocol reports an exact row-membership contract.
Composed domains and inner join equalities must cover every condition across the selected
transformation layers. Unsupported conditions (for example cross-column OR, LIKE, computed
predicates, HAVING, QUALIFY, LIMIT, OFFSET, FETCH, TABLESAMPLE, or non-inner joins) fail closed
with the residual reason, SQL clause, and originating layer. This also applies to every terminal
outcome in project-wide generation and to intermediate boundaries. Rejected rows are produced
only when the same exactness guarantee holds.

Some comparisons are exact only when the caller attests the database's comparison semantics.
For example, string filters can require binary collation, floating-point comparisons can require
no NaN values and equivalent signed zero, and timestamp filters may require a fixed session
timezone. Missing assumptions cause explicit errors that name the dependent conditions.

Pass declarations through the repeatable CLI option:

```console
sql-tdg generate \\
  --dialect generic \\
  --sql "SELECT name FROM customers WHERE name = 'Alice'" \\
  --schema 'customers:name=VARCHAR' \\
  --assume-comparison binary_collation \\
  --output .sql-tdg/customers
```

The accepted names are `binary_collation`, `no_char_padding`, `no_nan`,
`signed_zero_equivalent`, and `session_time_zone`. Declare only settings actually
guaranteed by the execution environment. Library callers can supply
`generate_classified_from_sql_with_assumptions`, the boundary-specific
`SqlGenerationSettings`, or call `AnalysisBundle::declare_comparison_assumptions`
before generating from a bundle. The declarations are embedded in the protocol snapshot
in `metadata.sqltdg`.

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

Use `--target <relation>` to generate for one named terminal outcome, or `--target-layer
<layer-id>` for an anonymous terminal outcome. Omit both to generate one shared dataset for every
terminal outcome, such as every model of a dbt project; the metadata then records an
`all-terminal-outcomes` target. By default the CLI materializes
physical source relations. Repeat `--boundary <relation>` to materialize explicitly selected
intermediate relations instead.

For dbt, declare every physical source column's `data_type` in your dbt YAML when the
source table is not yet present in the warehouse:

```yaml
version: 2
sources:
  - name: raw
    tables:
      - name: orders
        columns:
          - name: id
            data_type: INTEGER
          - name: amount
            data_type: BIGINT
```

Then compile the project and generate data without first creating the sources:

```console
dbt compile

sql-tdg generate \
  --dbt-project . \
  --target warehouse.analytics.customer_summary \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format parquet \
  --output .sql-tdg/customer-summary
```

The CLI reads `target/manifest.json` and uses `target/catalog.json` when it exists. Warehouse
catalog types take precedence over manifest-declared types. Without a catalog, every physical
dependency needs complete, typed column declarations in the manifest; missing schema or
`data_type` metadata fails explicitly. If a compiled model references an undeclared source
column, generation fails with the protocol's `unknown_schema_column` residual instead of
silently omitting that column. Use `--dbt-catalog <path>` to require a specific catalog file.

You can also pass artifacts directly:

```console
sql-tdg generate \
  --dbt-manifest target/manifest.json \
  --target warehouse.analytics.customer_summary \
  --output .sql-tdg/customer-summary
```

Add `--dbt-catalog target/catalog.json` to the direct-artifact invocation when a
specific warehouse catalog is required. The dbt adapter and dialect come from SQL Semantic
Protocol. The CLI does not interpret dbt SQL or artifact semantics itself.

Each successful run writes one deterministic relation file per generated relation plus
`metadata.sqltdg`. The metadata records the workload identity, selected target and boundary,
dialect, seed, generated relation row classifications, and the normalized SQL Semantic Protocol
snapshot needed to reproduce the generation inputs.

## Installation and releases

After the first stable release is published, install the CLI from crates.io with:

```console
cargo install sql-tdg
sql-tdg --version
```

Release Please owns version bumps, changelog entries, `vX.Y.Z` tags, and GitHub releases from
Conventional Commits. The bootstrap release is forced to `1.0.0`; generated release branches run
the full Rust and dbt acceptance checks, `cargo publish --dry-run --locked`, and an installed CLI
smoke test before the release can be published. The release workflow publishes to crates.io with
the repository `CARGO_REGISTRY_TOKEN` secret and can recover a tagged release that was not
published, while skipping versions already present on crates.io.

The one-time `release-as: 1.0.0` override must be removed after the initial stable release.

## Verification

Run the complete Rust verification suite with:

```console
make rust-checks
```

The dedicated dbt Core acceptance workflow requires dbt with the DuckDB adapter and then runs the
real fixture project through source generation, native `dbt seed`, model execution, intermediate
boundary generation, and result verification:

```console
python -m pip install -r tests/requirements-dbt-e2e.txt
make dbt-e2e
```

The same production CLI path is used outside the harness after normal dbt artifacts exist:

```console
dbt seed
dbt run
dbt docs generate

sql-tdg generate \
  --dbt-project . \
  --target '"warehouse"."analytics"."customer_summary"' \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format csv \
  --output .sql-tdg/customer-summary
```
