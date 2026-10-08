---
id: TASK-21.5
title: Generate from CTE and derived-table semantics once the protocol carries them
status: To Do
assignee: []
created_date: '2026-10-06 12:39'
updated_date: '2026-10-08 11:36'
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
- [ ] #1 sql-tdg depends on the protocol 2.x release that carries CTE and derived-table joins, predicates, and lineage
- [ ] #2 A filter inside a CTE or derived table constrains generated source rows
- [ ] #3 An inner equality join inside a CTE coordinates generated join keys
- [ ] #4 The maintainer's daily_revenue dbt model generates joined rows without uncoordinated keys
- [ ] #5 Unsupported local-relation shapes still fail explicitly
- [ ] #6 The CTE and derived-table guard is removed; generation through local relations relies only on protocol exactness and join equalities
- [ ] #7 DuckDB end-to-end tests run generated data through single CTEs, chained CTEs, derived tables, and a three-source CTE chain and confirm every matching row satisfies the query
- [ ] #8 Tests assert explicit errors for filters on computed, aggregate, window, or CASE local columns and for computed join keys inside CTEs
<!-- AC:END -->
