---
id: TASK-16
title: Add DuckDB materialization execution and result snapshots
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-1
dependencies:
  - TASK-15
---

## Description

Add DuckDB as the first executable test backend.

Materialize generated relations into a DuckDB database, execute the real SQL workload, capture an
approved expected result, and later verify the same workload against that expected result. This is
the behavior-regression layer: changing a predicate, join, grouping, projection, or other SQL
semantics should cause verification to fail unless the new result is explicitly approved.

## Acceptance Criteria

- [ ] Generated Arrow tables can be materialized into DuckDB with exact supported types and qualified relation names.
- [ ] A fresh deterministic database can be reproduced from the same test-case metadata and seed.
- [ ] The execution harness can run ordered multi-statement raw SQL workloads against the generated database.
- [ ] The harness can capture and persist an approved expected result for each selected outcome.
- [ ] Verification executes the current SQL and compares the complete result against the approved result.
- [ ] Result comparison is deterministic and defines handling for ordering, NULLs, decimals/floats, and nested values.
- [ ] A fixture proves that changing a filter to admit one previously rejected row fails verification.
- [ ] A fixture proves that making a filter stricter and losing an expected row fails verification.
- [ ] Database and snapshot writes are explicit outputs and never mutate source fixtures during read-only verification.
- [ ] DuckDB-specific code is isolated behind execution/materialization boundaries rather than leaking into solver semantics.
- [ ] Any new dependency is added only after the repository's dependency-approval rule is satisfied.
- [ ] `make rust-checks` remains green.
