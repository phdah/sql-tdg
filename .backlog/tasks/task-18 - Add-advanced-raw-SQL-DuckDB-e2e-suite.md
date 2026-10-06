---
id: TASK-18
title: Add advanced raw SQL DuckDB end-to-end suite
status: Done
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-17
---

## Description

Create a feature-rich raw SQL end-to-end fixture suite that exercises the complete production CLI
output and the repository's test-only DuckDB harness.

Use the fixtures in `phdah/sql-semantic-protocol` as the feature baseline, especially its advanced
bundle and dbt Core model set. The sql-tdg fixtures are execution tests rather than production
backend integrations: generate positive and negative data through the public CLI, load those outputs
inside the test harness, run the SQL, and compare the full output with an approved expectation.

## Progress

Implemented by the advanced raw SQL end-to-end suite in PR #24. The suite exercises production CLI
output through the test-only DuckDB harness across multi-layer relational transformations, windows,
set operations, intermediate boundaries, DML, deterministic matching/rejected generation, full
result assertions, and explicit unsupported paths.

## Acceptance Criteria

- [x] Include a multi-layer workload with named source, staging, intermediate, and terminal relations.
- [x] Cover joins, CASE-derived columns, aggregations/grouping/HAVING, subqueries/EXISTS, and multiple transformation layers.
- [x] Cover window functions and partition/order semantics, including a ranked/limited outcome where supported by DuckDB.
- [x] Cover UNION/UNION ALL, INTERSECT, EXCEPT, DISTINCT, ORDER BY, and LIMIT semantics where supported.
- [x] Cover representative DML such as INSERT and MERGE when both protocol semantics and DuckDB execution make the result well-defined.
- [x] Every fixture contains both data expected to contribute to the final result and data intentionally expected to be excluded.
- [x] At least one fixture is generated from raw physical sources and from an intermediate boundary.
- [x] The public CLI only writes Arrow-derived files; DuckDB loading and execution exist entirely inside test support.
- [x] Tests assert full expected result data, not only row counts or successful execution.
- [x] Mutation/regression cases prove meaningful SQL changes cause verification failure.
- [x] Unsupported protocol or DuckDB constructs are explicit test cases/errors, never silently skipped.
- [x] The suite is deterministic for a fixed seed.
- [x] `make rust-checks` remains green.
