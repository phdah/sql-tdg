---
id: TASK-31
title: Generate verifiable INSERT UPDATE DELETE and MERGE state transitions
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'sql-semantic-protocol TASK-65'
  - 'TASK-18'
  - 'TASK-19'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current tests cover INSERT and protocol models partial MERGE semantics, but sql-tdg cannot synthesize guaranteed pre-state and post-state transitions for richer incremental SQL. Depend on upstream TASK-65.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume protocol-declared initial-state requirements, mutation branches, effect domains and post-state constraints; no local SQL DML semantic parser.
- [ ] #2 Generate source and initial target tables for a safe subset of INSERT, UPDATE, DELETE and MERGE with deterministic touched/untouched rows.
- [ ] #3 Honor key/foreign-key constraints, matched/unmatched branches, source filters and exact after-state obligations; fail closed for unsupported effect semantics.
- [ ] #4 Execute real DuckDB mutations and assert before/after snapshots, idempotence where specified and intentionally nonmatching cases.
- [ ] #5 Document incremental-model capabilities, limitations and how exported files are loaded by callers.
<!-- AC:END -->
