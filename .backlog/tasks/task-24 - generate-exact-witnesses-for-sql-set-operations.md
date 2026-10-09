---
id: TASK-24
title: Generate exact witnesses for SQL set operations
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies:
  - TASK-37
  - TASK-43

references:
  - 'sql-semantic-protocol TASK-68'
  - 'sql-semantic-protocol TASK-69'
  - 'sql-semantic-protocol TASK-72'
  - 'sql-semantic-protocol TASK-85'
  - 'sql-semantic-protocol TASK-58'
  - 'TASK-18'
  - 'docs/usage.md'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
SQL Semantic Protocol models UNION/UNION ALL/INTERSECT/EXCEPT, but marks them residual for source-row exactness. Generate coordinated branch witnesses only after upstream TASK-58 exposes an exact branch-specific and multiplicity-aware contract.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume a released protocol contract for exact set operations; no SQL text parsing, branch inference or independent set-operation semantics in sql-tdg.
- [ ] #2 Generate reproducible source rows for UNION ALL, UNION, INTERSECT and EXCEPT where the protocol proves branch membership and duplicate handling.
- [ ] #3 Handle overlapping vs disjoint branches, duplicate elimination, NULL, input schemas and impossible constraints without mislabeling matching or rejected rows.
- [ ] #4 Add CLI and Rust API DuckDB E2E tests that verify final SQL results against documented expectations and explicit failures for residual shapes.
- [ ] #5 Update usage docs and feature matrix to distinguish fully supported set operation shapes from unsupported variants.
- [ ] #6 Expand fixture coverage to multi-column, nested, overlapping, branch-filtered, shared-source and intermediate set transformations using upstream exact construction; mark only explicitly out-of-scope variants residual.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.
