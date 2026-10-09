---
id: TASK-39
title: Generate DDL lifecycle and ordered stateful SQL workload fixtures
status: To Do
assignee: []
created_date: '2026-10-09'
updated_date: '2026-10-09'
labels: []
milestone: m-3
dependencies: 
  - TASK-37
  - TASK-40
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-80'
  - 'sql-semantic-protocol TASK-81'
  - 'sql-semantic-protocol TASK-82'
  - 'sql-semantic-protocol TASK-83'
  - 'sql-semantic-protocol TASK-84'
  - 'TASK-31'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Generator-side implementation for the single SQL Semantic Protocol 3.0.0 contract. This task must not reinterpret SQL, guess missing semantics or claim partial/local operator witnesses imply complete physical-source/output correctness. Validate with the pinned unpublished protocol release candidate before publication, then with the published crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 Consume protocol typed CREATE/REPLACE/ALTER/DROP/TRUNCATE, INSERT/UPDATE/DELETE/MERGE/UPSERT and scripted ordered before/after snapshots.
- [ ] #2 Generate deterministic initial physical sources and target prestate, untouched rows, mutation witnesses, schema changes, conflict choices and expected final states, without inventing unknown DML semantics.
- [ ] #3 Represent temporary and materialized objects, renamed/replaced relations, defaults/generated columns, constraint checks, transaction outcomes and dialect-specific conditional results where proved.
- [ ] #4 Export plan and fixture metadata usable by a caller's external database loader/test harness; keep CLI and generator free of SQL parsing or execution heuristics.
- [ ] #5 Verify full statement-by-statement DuckDB states and idempotence/negative cases; fail closed for unsupported effects or unverified dialect laws.
- [ ] #6 Document supported versus residual variants, add full-result differential tests and update docs/dialect-conformance.md for every changed feature.
<!-- AC:END -->
