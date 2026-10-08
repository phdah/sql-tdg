---
id: TASK-32
title: Generate reproducible scenarios for conflicting terminal outcomes
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'TASK-21.3'
  - 'TASK-19'
  - 'docs/usage.md'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Whole-project generation intersects all terminal constraints, and mutually contradictory model requirements yield ConflictingOutcomes. Add an opt-in scenario mode that partitions compatible outcomes rather than silently weakening any of them.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Keep existing all-outcomes single-dataset mode unchanged and still fail when no single shared dataset can meet the selected outcomes.
- [ ] #2 Provide a deterministic scenario partitioning mode with explicit outcome membership, distinct files/directories and metadata describing which terminal models each scenario satisfies.
- [ ] #3 Select scenarios from canonical protocol outcome and dependency graph semantics; do not reparse SQL or invent weakened constraints.
- [ ] #4 Test conflicting and compatible model groups, shared sources, seed reproducibility and independently executable dbt fixtures.
- [ ] #5 Expose a clear CLI/API option and document why scenarios are not one universal dataset.
<!-- AC:END -->
