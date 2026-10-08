# sql-tdg

[![Rust checks](https://github.com/phdah/sql-tdg/actions/workflows/rust-checks.yml/badge.svg)](https://github.com/phdah/sql-tdg/actions/workflows/rust-checks.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Synthetic data for real SQL workloads, not just isolated WHERE clauses.**

Randomly generated data often fails to exercise the query you want to test:
`WHERE` predicates filter out every row, join keys do not match, or dbt data tests
reject the inputs. **sql-tdg generates coherent source tables whose rows satisfy
the supported filters and inner equality joins of your SQL workload.** It also
honors supported keys, foreign-key relationships, and dbt data-test constraints
on generated relations.

Give it SQL or a compiled dbt project. Using
[SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol)
as its semantic boundary, sql-tdg chooses valid values and matching join keys,
then exports deterministic Arrow-backed data as Parquet or CSV. When it cannot
guarantee an exact match, it returns an explicit error instead of producing
misleading synthetic data. No database connection is required.

- **Semantic guarantees:** fail explicitly when query conditions cannot be represented exactly.
- **Reproducible data:** the same input, row counts, and seed yield the same generated rows.
- **Positive and negative cases:** request matching rows and, for supported selected outcomes,
  deliberately rejected rows.
- **Real-world workflows:** generate from raw SQL or compiled dbt artifacts, export Parquet or CSV,
  and inspect the accompanying `metadata.sqltdg` snapshot.

## SQL coverage

sql-tdg works across **multi-statement transformation pipelines**, not just simple
`SELECT` queries. The protocol analyzes SQL structure and resolves physical-source
conditions across transformation layers; sql-tdg uses the exact part of that
contract to generate consistent test data.

| Workload feature | What sql-tdg can generate against |
| --- | --- |
| Filters and joins | Typed `WHERE` domains, supported inner equality joins, and matching keys across relations |
| Multi-stage SQL | Multiple SQL statements, `CREATE VIEW` / `CREATE TABLE AS SELECT` layers, `WITH` (CTEs), and derived tables |
| Analytics | Queries projecting `GROUP BY` aggregates such as `COUNT` / `SUM` and window functions such as `ROW_NUMBER() OVER (PARTITION BY ... ORDER BY ...)` |
| Expressions | Projected `CASE` branches with source-column witnesses when the protocol proves them |
| dbt projects | Compiled model dependency graphs and supported source data tests, including uniqueness and foreign keys |

For example, the input workload can contain joins, a CTE, aggregation, and a
window expression together:

```sql
WITH qualifying_orders AS (
    SELECT o.customer_id, o.amount, c.segment
    FROM orders AS o
    JOIN customers AS c ON o.customer_id = c.id
    WHERE o.amount >= 100 AND c.active = TRUE
)
SELECT segment,
       COUNT(*) AS order_count,
       SUM(amount) AS total_amount,
       ROW_NUMBER() OVER (ORDER BY SUM(amount) DESC) AS revenue_rank
FROM qualifying_orders
GROUP BY segment
```

Here the generator targets the **source rows and join relationships** that
make the workload valid. The caller's SQL engine computes the resulting
aggregates and window values. sql-tdg does **not** guarantee a particular
aggregate total or window rank, and filtering on computed results (for example
`HAVING COUNT(*) > 2` or `QUALIFY revenue_rank = 1`) currently fails exactness
validation instead of being silently ignored. Set operations such as `UNION`
are likewise not yet guaranteed for exact generation.

### SQL dialects

Raw SQL uses `--dialect` (default: `generic`) and the dialect support of
[SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol).
Available dialects include:

**ANSI, BigQuery, ClickHouse, Databricks, DuckDB, Generic, Hive, Microsoft SQL
Server (`mssql`), MySQL, PostgreSQL (`postgresql` / `postgres`), Redshift,
Snowflake, and SQLite.**

dbt input uses its manifest's adapter dialect automatically. Dialect support
means the SQL can be parsed and analyzed using that dialect; **exact generation
still depends on the semantics of the particular query**, not just its dialect.
See [SQL scope and exactness](docs/usage.md#sql-scope-and-dialects).

## Contents

- [SQL coverage](#sql-coverage)
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
lossless representation of the supported Arrow types. The CLI defaults to 100
matching and 10 rejected rows per relation. Pass `--rejected 0` when generating
for all terminal outcomes or when rejected rows cannot be generated safely.

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

From a configured dbt project directory, compile the model SQL and generate
the catalog when the warehouse is available:

```console
dbt compile
dbt docs generate
sql-tdg generate --dbt-project . --matching 50 --rejected 0 --seed 42 --output sql-tdg-output
```

`dbt docs generate` builds `target/catalog.json` using warehouse metadata, including
column types for relations that already exist. For fresh projects whose physical
sources do not yet exist in the warehouse, the catalog may be empty or
incomplete; declare every physical source column's `data_type` in source YAML
so generation can use the compiled manifest instead.

sql-tdg reads `target/manifest.json` and, when present, `target/catalog.json`.
Catalog types take precedence over the source YAML's `data_type` declarations.
With no `--target`, it generates **one shared physical-source dataset** satisfying
all compatible terminal models. This whole-project mode requires
`--rejected 0` because deliberately rejected rows require a single selected
terminal outcome. Conflicting outcomes fail explicitly instead of producing
falsely labeled data.

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
