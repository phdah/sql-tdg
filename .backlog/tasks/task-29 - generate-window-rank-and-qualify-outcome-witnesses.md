---
id: TASK-29
title: Generate window rank and QUALIFY outcome witnesses
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'sql-semantic-protocol TASK-60'
  - 'TASK-18'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Window expressions are analyzable, but sql-tdg does not guarantee which source rows survive ROW_NUMBER/RANK-based filters and currently rejects QUALIFY. Consume protocol TASK-60 to plan partitions and rank witnesses.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume canonical partition, ordering, rank and tie constraints from a released protocol without parsing window SQL locally.
- [ ] #2 Generate matching sources for supported ROW_NUMBER() = 1 / <= N and qualifying window filters, including deterministic partition sizes and order keys.
- [ ] #3 Handle ties, NULL order keys, and differing dialect comparison/order assumptions; fail closed where result rank is not provable.
- [ ] #4 Execute representative window models and QUALIFY queries in DuckDB and assert the exact surviving rows and rejected witnesses.
- [ ] #5 Document supported window functions and excluded frames/filter shapes.
<!-- AC:END -->
