---
id: TASK-16
title: Add DuckDB materialization execution and result snapshots
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-15
---

## Description

Add DuckDB as the first executable test backend.

Materialize generated relations into a DuckDB database, execute the real SQL workload, capture an
approved expected result, and later verify the same workload against that expected result. This is
the behavior-regression layer: changing a predicate, join, grouping, projection, or other SQL
semantics should cause verification to fail unless the new result is explicitly approved.

## Progress

Added an isolated DuckDB execution boundary that materializes protocol-backed generated tables,
executes ordered raw SQL workloads, canonicalizes complete results, persists explicit approval
snapshots, and verifies current workload output without implicit re-approval. File-backed databases
are explicit outputs and can be reopened read-only for verification. Integration coverage exercises
qualified relations, shared TASK-12 datatypes, deterministic regeneration, multi-statement
execution, and both widening and narrowing predicate regressions.

## Acceptance Criteria

- [x] Generated Arrow tables can be materialized into DuckDB with exact supported types and qualified relation names.
- [x] Arrow-to-DuckDB round-trip tests prove every TASK-12 datatype supported by both systems preserves type and value semantics.
- [x] A fresh deterministic database can be reproduced from the same test-case metadata and seed.
- [x] The execution harness can run ordered multi-statement raw SQL workloads against the generated database.
- [x] The harness can capture and persist an approved expected result for each selected outcome.
- [x] Verification executes the current SQL and compares the complete result against the approved result.
- [x] Result comparison is deterministic and defines handling for ordering, NULLs, decimals/floats, and nested values.
- [x] A fixture proves that changing a filter to admit one previously rejected row fails verification.
- [x] A fixture proves that making a filter stricter and losing an expected row fails verification.
- [x] Database and snapshot writes are explicit outputs and never mutate source fixtures during read-only verification.
- [x] DuckDB-specific code is isolated behind execution/materialization boundaries rather than leaking into solver semantics.
- [x] Any new dependency is added only after the repository's dependency-approval rule is satisfied.
- [x] `make rust-checks` remains green.
