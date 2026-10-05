---
id: TASK-16
title: Add test-only DuckDB execution and backend-neutral file exports
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-15
---

## Description

Keep sql-tdg's production boundary backend-neutral while adding the execution machinery required to
test generated data end to end.

Generated relations remain Arrow-backed library outputs and can be exported as CSV or Parquet.
sql-tdg must not connect to, create, seed, or mutate a user's database. DuckDB exists only as a
repository test harness so real SQL can be executed against generated data and compared with
explicit expected-result snapshots.

## Progress

Added Arrow RecordBatch, CSV, and Parquet export helpers as the production persistence boundary.
Moved DuckDB to a dev dependency and removed DuckDB execution from the public library API. The
test-only DuckDB harness materializes protocol-backed generated tables, executes ordered raw SQL
workloads, canonicalizes complete results, persists explicit approval snapshots, and verifies
current workload output without implicit re-approval. Nested DuckDB values are constructed from
scalar bind parameters because duckdb-rs does not bind LIST/ARRAY/STRUCT/MAP values directly.

## Acceptance Criteria

- [x] Generated tables remain backend-neutral Arrow-backed library values.
- [x] Finalized tables can be exposed as Arrow RecordBatch values without database-specific code.
- [x] Flat generated tables can be exported as CSV for common interoperability workflows.
- [x] Generated tables can be exported as Parquet with Arrow schema metadata preserved.
- [x] DuckDB is a dev-only dependency and no DuckDB executor is exposed from the production library API.
- [x] The test harness can materialize generated relations into DuckDB with exact supported types and qualified relation names.
- [x] Arrow-to-DuckDB round-trip tests prove every TASK-12 datatype supported by both systems preserves type and value semantics.
- [x] A fresh deterministic database can be reproduced from the same test-case metadata and seed.
- [x] The execution harness can run ordered multi-statement raw SQL workloads against the generated database.
- [x] The harness can capture and persist an approved expected result for each selected outcome.
- [x] Verification executes the current SQL and compares the complete result against the approved result.
- [x] Result comparison is deterministic and defines handling for ordering, NULLs, decimals/floats, and nested values.
- [x] Fixtures prove both widening and narrowing predicate changes fail verification.
- [x] DuckDB-specific code remains test-only and never leaks into protocol, solver, generation, or public storage APIs.
- [x] New dependencies are limited to Arrow CSV/Parquet export support plus the already-approved dev-only DuckDB harness.
- [x] `make rust-checks` remains green.
