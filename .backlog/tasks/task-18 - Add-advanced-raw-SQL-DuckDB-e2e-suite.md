---
id: TASK-18
title: Add advanced raw SQL DuckDB end-to-end suite
status: To Do
assignee: []
created_date: '2026-10-04'
labels: []
milestone: m-2
dependencies:
  - TASK-17
---

## Description

Create a feature-rich raw SQL end-to-end fixture suite that exercises the complete CLI and DuckDB
workflow.

Use the fixtures in `phdah/sql-semantic-protocol` as the feature baseline, especially its advanced
bundle and dbt Core model set. The sql-tdg fixtures should be execution tests rather than copies of
protocol unit tests: generate positive and negative data, materialize it, run the SQL, and compare
the full output with an approved expectation.

## Acceptance Criteria

- [ ] Include a multi-layer workload with named source, staging, intermediate, and terminal relations.
- [ ] Cover joins, CASE-derived columns, aggregations/grouping/HAVING, subqueries/EXISTS, and multiple transformation layers.
- [ ] Cover window functions and partition/order semantics, including a ranked/limited outcome where supported by DuckDB.
- [ ] Cover UNION/UNION ALL, INTERSECT, EXCEPT, DISTINCT, ORDER BY, and LIMIT semantics where supported.
- [ ] Cover representative DML such as INSERT and MERGE when both protocol semantics and DuckDB execution make the result well-defined.
- [ ] Every fixture contains both data expected to contribute to the final result and data intentionally expected to be excluded.
- [ ] At least one fixture is executed from raw physical sources and from an intermediate boundary.
- [ ] Tests assert full expected result data, not only row counts or successful execution.
- [ ] Mutation/regression cases prove meaningful SQL changes cause verification failure.
- [ ] Unsupported protocol or DuckDB constructs are explicit test cases/errors, never silently skipped.
- [ ] The suite is deterministic for a fixed seed.
- [ ] `make rust-checks` remains green.
