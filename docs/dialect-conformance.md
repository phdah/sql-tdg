# SQL dialect conformance

The `--dialect` option selects a **SQL Semantic Protocol parser and analysis dialect**.
It is not a promise that sql-tdg can generate witnesses for every construct in
that dialect, nor that the generated data has been executed by its database engine.
sql-tdg accepts only the exact source-row constraints provided by the protocol;
residual conditions fail closed.

## Evidence levels

- **P**: Protocol parsing and analysis completed for the named fixture.
- **G**: sql-tdg generated physical-source rows and Rust assertions checked the
  source-domain or inner-join witness, including deterministic repetition where
  applicable. This does not certify the SQL's downstream query output.
- **R**: Protocol analysis recognized an unsupported row-membership condition and
  generation explicitly refused it.
- **E**: The generated files were loaded into an execution engine and the SQL
  result was checked. Only **DuckDB** has repository execution tests.

The cross-dialect fixtures in [`tests/dialect_conformance.rs`](../tests/dialect_conformance.rs)
exercise these shared shapes for all dialects: bounded integer filters (P/G),
inner equality joins (P/G), filter propagation through CTEs (P/G),
projection-only `COUNT`/`GROUP BY` (P/G for source rows only), and
aggregate-dependent `HAVING` (P/R). The tests also exercise quoted identifiers
with BigQuery, MySQL, SQL Server, and PostgreSQL syntax.

| Protocol dialect | Scalar filter | Inner equality join | CTE filters | Projected aggregate | HAVING membership | Engine E2E |
| --- | --- | --- | --- | --- | --- | --- |
| ansi | P/G | P/G | P/G | P/G | P/R | Not run |
| bigquery | P/G | P/G | P/G | P/G | P/R | Not run |
| clickhouse | P/G | P/G | P/G | P/G | P/R | Not run |
| databricks | P/G | P/G | P/G | P/G | P/R | Not run |
| duckdb | P/G | P/G | P/G | P/G | P/R | DuckDB only |
| generic | P/G | P/G | P/G | P/G | P/R | Not run |
| hive | P/G | P/G | P/G | P/G | P/R | Not run |
| mssql | P/G | P/G | P/G | P/G | P/R | Not run |
| mysql | P/G | P/G | P/G | P/G | P/R | Not run |
| postgresql (alias: postgres) | P/G | P/G | P/G | P/G | P/R | Not run |
| redshift | P/G | P/G | P/G | P/G | P/R | Not run |
| snowflake | P/G | P/G | P/G | P/G | P/R | Not run |
| sqlite | P/G | P/G | P/G | P/G | P/R | Not run |

## Additional coverage and limits

- **Execution:** [`tests/advanced_raw_sql.rs`](../tests/advanced_raw_sql.rs)
  and [`tests/dbt_core_e2e.rs`](../tests/dbt_core_e2e.rs) execute DuckDB
  fixtures; those results do not certify another engine's collation, null
  treatment, timestamp interpretation, or dialect-specific SQL.
- **dbt constraints:** dbt manifest/catalog data-test semantics are validated
  through the protocol in the dbt fixtures, not independently in each dialect.
  The table above does not assert dialect-level constraint coverage.
- **Other negative cases:** [`tests/exactness.rs`](../tests/exactness.rs)
  covers residual constructs, including cross-column OR, computed filters,
  LIMIT and QUALIFY, and explicit comparison assumptions. A parser accepting a
  construct does not make it an exact generation contract.
- **Attested comparison settings:** binary collation, character-padding, NaN,
  signed-zero, and session-timezone assumptions are caller-supplied. This
  repository cannot infer them from a dialect label. See
  [Comparison assumptions](usage.md#comparison-assumptions).
- **Dialect-specific syntax:** quoted identifier fixtures cover four dialects.
  Untested syntax variants and all engines other than DuckDB remain
  *unverified*, not implicitly supported.

## Maintaining the matrix

For every new SQL semantic feature or dialect in this milestone, update the
fixture set and the matrix **in the same PR**. Assert concrete generated source
values or an explicit residual error. Separate parse/analysis, generation, and
actual engine execution evidence. Add a dialect-specific execution claim only
when a test runs its generated data through that engine and checks the SQL
result. This matrix is an evidence record, not a hardcoded production dialect
whitelist.
