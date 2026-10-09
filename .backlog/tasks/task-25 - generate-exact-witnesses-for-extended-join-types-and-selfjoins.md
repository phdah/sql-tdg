---
id: TASK-25
title: Generate exact witnesses for extended join types and self-joins
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
  - 'sql-semantic-protocol TASK-71'
  - 'sql-semantic-protocol TASK-61'
  - 'TASK-14'
  - 'TASK-18'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The generator coordinates supported inner equality join keys, but outer, semi, anti, non-equality and repeated/self-join conditions currently fail exactness. Add safe matched and unmatched row planning from upstream TASK-61.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume canonical protocol match/non-match, null-extension and relation-instance obligations without independently interpreting SQL joins.
- [ ] #2 Generate rows for a documented safe subset of LEFT/RIGHT/FULL, SEMI and ANTI joins, non-equality predicates and repeated/self-joins as expressible in the protocol.
- [ ] #3 Preserve cardinality, nullability, keys, foreign keys, duplicate relations, and deterministic matching/rejected classification; impossible shape gives an actionable error.
- [ ] #4 Verify complete SQL results with DuckDB across missing parent, unmatched child, NULL join key, multi-match and alias cases.
- [ ] #5 Document remaining unsupported join shapes and assumptions.
- [ ] #6 Include composite/non-equi/outer join trees and filtered producer relations; prove matching, unmatched, anti and self-join behavior on complete source graphs.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.
