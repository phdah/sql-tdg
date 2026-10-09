---
id: TASK-30
title: Support requested output cardinality and distributions
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies:
  - TASK-24
  - TASK-28
  - TASK-29
  - TASK-37
  - TASK-38
  - TASK-43
references:
  - 'sql-semantic-protocol TASK-69'
  - 'sql-semantic-protocol TASK-85'
  - 'sql-semantic-protocol TASK-64'
  - 'TASK-24'
  - 'TASK-28'
  - 'TASK-29'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The CLI --matching and --rejected describe generated source rows, not final query cardinality. Add explicit optional goals that can be guaranteed by the protocol's outcome cardinality contract (TASK-64).

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Define a library and CLI contract distinguishing source row count from requested output row count, group count, value distribution and cardinality bounds.
- [ ] #2 Consume only canonical protocol output-goal constraints; generate feasible goals deterministically and return an actionable unsatisfiable-goal error.
- [ ] #3 Handle multiplicity from supported joins, grouping, DISTINCT and compatible set/window capabilities without claiming unsupported shapes.
- [ ] #4 Validate target counts and distributions by executing fixture SQL in DuckDB; include a negative impossible-goal test.
- [ ] #5 Document exact scope and guarantee level, with backwards-compatible defaults.
- [ ] #6 Guarantee requested output counts and typed distributions through cross-operator and multi-column composed pipelines, including shared-source and impossible goals.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.
