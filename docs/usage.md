# Usage and semantic guarantees

sql-tdg generates reproducible relation data from a
[SQL Semantic Protocol](https://github.com/phdah/sql-semantic-protocol) analysis.
It does not connect to a database or execute the input SQL. The generated
Parquet/CSV files can be loaded by a separate test harness or application.

## SQL scope and dialects

SQL analysis runs through SQL Semantic Protocol, which recognizes
`ansi`, `bigquery`, `clickhouse`, `databricks`, `duckdb`,
`generic`, `hive`, `mssql`, `mysql`, `postgresql` (also
`postgres`), `redshift`, `snowflake`, and `sqlite`.
Use `--dialect <name>` for raw SQL; dbt artifacts supply their
adapter dialect. Each dialect can have different syntax, and not
every feature or construct has exact generation semantics.

SQL Semantic Protocol tracks many kinds of transformations: multiple
statements, named output relations, query-backed DDL, CTEs and derived
tables, joins, CASE expressions, grouping, aggregate expressions,
window definitions, and set operations. sql-tdg consumes the
**composed physical-source conditions** from that analysis. For example:

- Supported filters and inner equality relationships can become source
  domains and coordinated join-key values, including through chains
  of CTEs and named materialization layers.
- `GROUP BY`, `COUNT`, `SUM`, and
  `ROW_NUMBER() OVER (PARTITION BY ... ORDER BY ...)` may appear in
  projection-only analytic outputs. Generated sources can execute those
  queries, but sql-tdg does not set their computed output totals or
  ranking results.
- Aggregate-dependent `HAVING`, window-dependent `QUALIFY`, and
  filtering on other derived expressions are residual conditions when
  the protocol cannot reduce them exactly to source-row constraints.
- Although the protocol can describe `UNION`, `INTERSECT`, and
  `EXCEPT`, sql-tdg does not claim exact generation for unsupported
  set-operation conditions. Unknown row-membership effects fail closed.

Generation operates on source tables or a selected intermediate
boundary. It does **not** generate final query results or prove all
possible database semantics. Use the [exactness contract](#classification-and-exactness)
to distinguish a construct present in the input from a condition
the tool can guarantee.

## Raw SQL

Pass one or more inline queries with `--sql`, files with `--file`, and
typed physical source columns with repeated `--schema`:

```console
sql-tdg generate \
  --dialect generic \
  --sql 'SELECT amount FROM orders WHERE amount >= 10 AND amount < 20' \
  --schema 'orders:amount=INTEGER' \
  --matching 50 \
  --rejected 10 \
  --seed 42 \
  --format parquet \
  --output sql-tdg-output
```

Columns not constrained by the protocol get varied, moderate values; constrained
scalar ranges are sampled across their allowed intervals. Use a fixed `--seed`
to reproduce the same samples. The CLI defaults to 100 matching rows, 10
rejected rows, seed 42, and Parquet output. Pass `--rejected 0` when
rejected data cannot be generated safely.

## dbt projects

Run `dbt compile` first so the project has a current `target/manifest.json`.
Run `dbt docs generate` as well when a database connection is available to
produce `target/catalog.json` with warehouse-introspected column types.
Then pass `--dbt-project` to point to the directory containing `target/`:

```console
dbt compile
dbt docs generate
sql-tdg generate --dbt-project . --matching 50 --rejected 0 --seed 42 --output sql-tdg-output
```

A catalog is not a prerequisite for generating sources that do not yet exist
in the warehouse. In that case it may have no entries for those relations,
and the compiled manifest must provide complete `data_type` declarations
for every required physical source column.
The CLI uses `target/catalog.json` if it exists, and otherwise uses complete
`data_type` declarations on source columns in the compiled manifest. Catalog
types win when both are present. Missing or inconsistent source type evidence
causes an actionable error; sql-tdg does not infer it from SQL.

The `--dbt-manifest` option accepts an explicit artifact path; combine it with
`--dbt-catalog` when the catalog is required. The SQL dialect comes from the
dbt manifest adapter, not a CLI `--dialect` override. Compiled model SQL is the
source of query semantics; raw dbt Jinja/model text is never parsed by sql-tdg.

Tests and dbt constraints are consumed through the protocol, including unique
and composite keys, not-null, accepted values, and supported foreign keys.
Self-references are supported when they can be generated consistently.
Unsupported test configurations, constraint diagnostic codes, and conflicting
domains produce explicit errors. Constraints on **nongenerated** model
relations are printed as `status=not_honored` rather than claimed to be
satisfied. Singular tests and arbitrary model-specific totals are not
synthesized into test fixtures.

### Outcome selection

Without a selector, sql-tdg generates one shared set of physical sources whose
rows satisfy **every** terminal outcome. Use `--rejected 0` in this mode:
the CLI otherwise defaults to 10 rejected rows, which require one selected
terminal outcome. If two terminal models demand mutually
exclusive values from the same source, generation fails with
`ConflictingOutcomes` and identifies the affected outcomes.

Use `--target` to select one named terminal relation or `--target-layer`
for an anonymous layer. A named dbt target must match the protocol's exact
canonical relation identity, including quotation when present in
`manifest.json` (for example
`"fixture"."raw_analytics"."final_orders"` in the committed dbt fixture).

### Independent scenarios

If terminal models have incompatible source conditions, pass `--scenarios`
instead of weakening the requirements of any model:

```console
sql-tdg generate --dbt-project . --scenarios --matching 50 --rejected 0 --seed 42 --output sql-tdg-scenarios
```

Each `scenario-0001/`, `scenario-0002/`, etc. contains its own source files and
`metadata.sqltdg` with a `scenario-outcomes` target identifying the exact
canonical terminal outcome members. The CLI also prints each group's membership.
Scenario outputs are **independent test runs**, not source partitions intended to
be loaded together. Run downstream dbt models against one scenario's sources at
a time. Use a new or empty output directory to prevent stale scenarios.

Partitioning is deterministic first-fit by canonical outcome identity. Each
new outcome joins the first group for which the existing exact shared-outcomes
generator proves compatibility; it may not find the smallest number of groups.
An unsupported or individually unsatisfiable outcome still fails the entire
command. This is opt-in: without `--scenarios`, the existing single-dataset
behavior and `ConflictingOutcomes` error remain unchanged. Scenario mode
requires `--rejected 0` and cannot combine with `--target`, `--target-layer`,
or `--boundary`. The Rust API offers `generate_scenarios_from_bundle` on an
already analyzed protocol bundle.

Repeat `--boundary` to materialize named intermediate relations instead of
physical sources for a selected outcome. Only select a boundary if the protocol
resolves its semantics exactly. Rejected rows and intermediate boundaries
require selecting a single outcome instead of whole-project mode.

### Loading and verifying

Exported files are backend-neutral. You are responsible for importing them
into the test database or providing them as dbt seeds, then running `dbt build`
or your SQL test harness. The repository's `make dbt-e2e` target demonstrates
the full generation, load, execution, and verification sequence using DuckDB
and the committed fixture; it does not alter production databases.

## Classification and exactness

Matching rows satisfy the selected protocol conditions. For `--rejected N`,
sql-tdg deliberately violates one supported scalar condition or breakable
relationship while preserving all the remaining conditions. If it cannot
guarantee that classification, it returns an explicit error rather than
emitting misleading rejected rows.

Every relevant transformation layer must provide an **exact row-membership
contract**. The protocol must represent the composed predicates and
inner-join equalities across CTEs and derived tables. Examples that may prevent
exact generation include:

- cross-column OR and LIKE predicates;
- computed-column filters and computed join keys;
- HAVING, QUALIFY, LIMIT, OFFSET, FETCH, and TABLESAMPLE;
- non-inner joins and unresolvable transformation lineage.

These are not merely ignored: errors identify the residual condition, clause,
or originating layer where the protocol can supply that information. The same
exactness rule applies in whole-project mode, for selected outcomes, and for
rejected-row generation.

Projected CASE branches are exercised when the protocol supplies safe
physical-source branch domains. The library exposes `case_coverage()`, and
the CLI prints `case_branch=...` with `covered`, `unreachable`,
`unknown`, or `insufficient_rows`. Aggregate and unsafe-lineage CASE
expressions may be reported as `unknown`, not silently declared covered.

## Comparison assumptions

A protocol condition may require a caller-attested comparison setting.
Attest only properties guaranteed by the target SQL execution environment:

| Setting | When it matters |
| --- | --- |
| `binary_collation` | String comparisons use binary collation |
| `no_char_padding` | Text comparisons have no blank-padding behavior |
| `no_nan` | Floating-point comparisons exclude NaN |
| `signed_zero_equivalent` | Floating-point signed-zero semantics are equivalent |
| `session_time_zone` | Timestamp comparisons use a fixed session timezone |

For example, when binary collation is guaranteed:

```console
sql-tdg generate \
  --dialect generic \
  --sql "SELECT name FROM customers WHERE name = 'Alice'" \
  --schema 'customers:name=VARCHAR' \
  --assume-comparison binary_collation \
  --output sql-tdg-output
```

Repeat `--assume-comparison` for additional attested settings. The library
also accepts explicit comparison assumptions through
`generate_classified_from_sql_with_assumptions`,
`SqlGenerationSettings`, or
`AnalysisBundle::declare_comparison_assumptions`.

## Formats and reproducibility

Parquet is the default and supports the broadest lossless Arrow-backed types:
integers, floating point, decimals, temporal types, strings, binary, nullable
values, arrays, maps, structs, and JSON-like storage where their canonical
protocol datatypes can be represented without loss. Unrepresentable types
fail with `UnsupportedSourceType`; they are not silently coerced. CSV is
intended for flat interoperability, including dbt seed workflows.

Each successful CLI run emits:

- one deterministically named file per generated relation;
- `metadata.sqltdg` containing workload identity, selection, boundaries, seed,
  row classifications, and the normalized protocol snapshot;
- CLI output describing generated paths, CASE coverage, and constraints that
  were not honored because their relations were not generated.

With identical analysis inputs, source schemas, row counts, and seed, generated
data is deterministic. A separate database harness is responsible for
executing SQL and comparing downstream results.
