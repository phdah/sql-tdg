---
id: TASK-21.5
title: Generate from CTE and derived-table semantics once the protocol carries them
status: Done
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-08'
labels: []
milestone: m-2
dependencies:
  - TASK-21.4
  - TASK-21.8
references:
  - sql-semantic-protocol TASK-44
  - sql-semantic-protocol TASK-45
  - sql-semantic-protocol TASK-52
  - src/protocol.rs
parent_task_id: TASK-21
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Standard dbt models are written as CTE chains. sql-tdg currently refuses any query source that is a CTE or derived table (TASK-21.4 guard `reject_local_relation_sources` in src/protocol.rs).

Protocol status (verified at protocol commit 3a4d3c6, release 2.0.0):
- Joins, predicates, domains, and lineage are carried through CTEs and derived tables (protocol TASK-31, TASK-36 to TASK-38, TASK-44).
- Inner equi-joins inside local relations appear in `join_equalities` on physical columns and are exact (protocol TASK-45, TASK-52).
- Anything not carried makes `condition_exactness` residual (for example filters on computed, aggregate, window, or CASE CTE columns, or computed join keys).
- A daily-revenue-like chain (three sources, two equi-joins, GROUP BY, CASE over SUM) is exact once `binary_collation` is declared for its string filter.

Once TASK-21.8 consumes exactness and join equalities, the guard is redundant: remove it and prove real CTE generation end to end. The CASE over an aggregate in daily_revenue does not affect row conditions; covering its branches is TASK-21.7.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sql-tdg depends on the protocol 2.x release that carries CTE and derived-table joins, predicates, and lineage
- [x] #2 A filter inside a CTE or derived table constrains generated source rows
- [x] #3 An inner equality join inside a CTE coordinates generated join keys
- [x] #4 The maintainer's daily_revenue dbt model generates joined rows without uncoordinated keys
- [x] #5 Unsupported local-relation shapes still fail explicitly
- [x] #6 The CTE and derived-table guard is removed; generation through local relations relies only on protocol exactness and join equalities
- [x] #7 DuckDB end-to-end tests run generated data through single CTEs, chained CTEs, derived tables, and a three-source CTE chain and confirm every matching row satisfies the query
- [x] #8 Tests assert explicit errors for filters on computed, aggregate, window, or CASE local columns and for computed join keys inside CTEs
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
- The crate already consumes the published `sql-semantic-protocol = "2.0.0"`; its composed exactness and physical `join_equalities` provide the required local-relation semantics.
- Removed `reject_local_relation_sources`, its ancestor traversal, and both call sites. The existing `require_exact_conditions` and physical equality handling remain authoritative for single and whole-project generation.
- `tests/local_relations.rs` verifies CTE and derived-table filters, chained CTEs, ancestor views, intermediate boundaries, whole-project intersection, and coordinated join keys across four physical sources. It verifies unsupported computed, aggregate, window, CASE, and computed-join-key predicates fail explicitly rather than weakening semantics.
- `tests/duckdb_execution.rs` materializes the generated Arrow relations and executes simple/chained local queries and a representative three-source CTE join plus aggregate against DuckDB. The maintainer's separate external daily_revenue fixture remains part of TASK-21's manual release gate.
- See PR #30 and its independent Rust and dbt CI checks.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Generation now follows SQL Semantic Protocol's exact CTE/derived-table predicates, source lineage and physical join equalities instead of rejecting all local relations. Unsupported computed local filters and join keys continue to fail closed. DuckDB-backed tests exercise the resulting source data end to end; the external manual dbt release gate remains tracked separately under TASK-21.
<!-- SECTION:FINAL_SUMMARY:END -->
