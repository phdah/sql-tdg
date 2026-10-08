# sql-tdg

[![Rust checks](https://github.com/phdah/sql-tdg/actions/workflows/rust-checks.yml/badge.svg)](https://github.com/phdah/sql-tdg/actions/workflows/rust-checks.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Generate deterministic, Arrow-backed test data for SQL queries and dbt projects without connecting to a database.**

sql-tdg uses [SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol)
to resolve query conditions, source schemas, and dbt data-test constraints. It generates
source or intermediate relation files that satisfy the supported semantics, so you can
exercise real transformations with controlled data rather than hand-maintaining fixtures.

- **Semantic guarantees:** fail explicitly when query conditions cannot be represented exactly.
- **Reproducible data:** the same input, row counts, and seed yield the same generated rows.
- **Positive and negative cases:** request matching rows and, for supported selected outcomes,
  deliberately rejected rows.
- **Real-world workflows:** generate from raw SQL or compiled dbt artifacts, export Parquet or CSV,
  and inspect the accompanying `metadata.sqltdg` snapshot.

## Contents

- [Install](#install)
- [Quick start](#quick-start)
- [Use with dbt](#use-with-dbt)
- [Use as a Rust library](#use-as-a-rust-library)
- [Guarantees and limitations](#guarantees-and-limitations)
- [Documentation and contributing](#documentation-and-contributing)

## Install

Install the CLI from a checkout of this repository:

```console
cargo install --path .
sql-tdg --help
```

Once the first stable release is published on crates.io, the published package can be
installed with `cargo install sql-tdg`.

## Quick start

Generate 50 matching and 10 rejected `orders` rows for a raw SQL filter:

```console
sql-tdg generate \
  --dialect generic \
  --sql 'SELECT amount FROM orders WHERE amount >= 10 AND amount < 20' \
  --schema 'orders:amount=INTEGER' \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format csv \
  --output sql-tdg-output
```

The command writes `sql-tdg-output/0001-orders.csv` and
`sql-tdg-output/metadata.sqltdg`. The first 50 rows satisfy the filter, and the
10 rejected rows fall outside it. Use `--format parquet` (the default) for a
lossless representation of the supported Arrow types.

Multiple `--sql`, `--file`, and `--schema` options are accepted. The
`--schema` form is `RELATION:COLUMN=SQL_TYPE`; SQL types are normalized by the
protocol for the selected dialect.

## Use with dbt

Declare physical source column types in your dbt source YAML, particularly if the
warehouse tables do not exist yet:

```yaml
version: 2
sources:
  - name: raw
    tables:
      - name: orders
        columns:
          - name: id
            data_type: BIGINT
            data_tests:
              - unique
              - not_null
          - name: amount
            data_type: INTEGER
```

From a configured dbt project directory:

```console
dbt compile
sql-tdg generate --dbt-project . --matching 50 --seed 42 --output sql-tdg-output
```

sql-tdg reads `target/manifest.json` and, when present, `target/catalog.json`.
Catalog types take precedence over the source YAML's `data_type` declarations.
With no `--target`, it generates **one shared physical-source dataset** satisfying
all compatible terminal models. Conflicting outcomes fail explicitly instead of
producing falsely labeled data.

Supported dbt generic data tests and constraints (unique, not_null, accepted_values,
and relationships) are enforced **on generated relations** through the protocol.
Constraints on models that are not being generated are reported as not honored.
Generation does not execute `dbt build` or load files into your warehouse; that
remains the caller's responsibility.

See [dbt options and limitations](docs/usage.md#dbt-projects) for selecting a single
model, intermediate boundaries, and missing catalog schemas.

## Use as a Rust library

```rust
use sql_tdg::{RelationSchema, SchemaColumn, generate_from_sql};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema = RelationSchema::new(
        "orders",
        vec![SchemaColumn::from_sql_type("amount", "INTEGER", "generic")?],
    )?;

    let generated = generate_from_sql(
        "SELECT amount FROM orders WHERE amount >= 10 AND amount < 20",
        "generic",
        &[schema],
        100,
        42,
    )?;

    let amounts = generated
        .table("orders")
        .ok_or_else(|| std::io::Error::other("orders source was not generated"))?
        .get_ints("amount")?
        .ok_or_else(|| std::io::Error::other("amount column was not generated"))?;

    assert!(amounts.iter().all(|value| (10..20).contains(value)));
    Ok(())
}
```

Library callers can also generate from an `AnalysisBundle`, select outcomes or
boundaries, request classified rows, and consume Arrow tables directly.

## Guarantees and limitations

sql-tdg **does not parse or reinterpret SQL**. SQL Semantic Protocol supplies
the canonical semantic contract. Generation succeeds only when row membership
is exact for the selected outcome or set of outcomes.

| Supported when fully described by the protocol | Fails closed or is reported |
| --- | --- |
| Scalar value domains, ranges, nullability, and typed source schemas | Missing schemas, contradictory domains, unknown types |
| Supported inner equality joins, CTEs, and derived tables | Non-inner joins, computed join keys, unresolved composition |
| Primary/unique keys, foreign keys, not-null, accepted values on generated relations | Unsupported dbt tests or constraint diagnostics |
| Reachable projected CASE branches with physical-source witnesses | Aggregate/computed CASE branches whose coverage cannot be proven (reported as unknown) |
| Binary-collated strings and other sensitive comparisons with caller-attested assumptions | Missing comparison assumptions or non-exact conditions such as LIKE, cross-column OR, HAVING, QUALIFY, LIMIT, OFFSET |

The generator is **not** a database client or a query-execution engine. Database
execution in this repository exists only in the DuckDB end-to-end test harness.

For a detailed description of exactness, comparison assumptions, data types,
output metadata, and rejected-row restrictions, see [Usage and semantics](docs/usage.md).

## Documentation and contributing

- [Usage and semantics](docs/usage.md)
- [Contributor and architecture conventions](AGENTS.md)
- [Changelog](CHANGELOG.md)
- [Issue tracker](https://github.com/phdah/sql-tdg/issues)

Run `make rust-checks` before opening a pull request, and `make doc` when
public API documentation changes. The [Makefile](Makefile) defines verification
commands; [AGENTS.md](AGENTS.md) defines development conventions.

## License

Licensed under the [MIT License](LICENSE).
