---
id: TASK-25
title: Generate exact witnesses for extended join types and self-joins
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
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
<!-- AC:END -->
