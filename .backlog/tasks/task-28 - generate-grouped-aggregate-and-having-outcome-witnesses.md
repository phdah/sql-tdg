---
id: TASK-28
title: Generate grouped aggregate and HAVING outcome witnesses
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies:
  - TASK-37
  - TASK-38
  - TASK-43

references:
  - 'sql-semantic-protocol TASK-68'
  - 'sql-semantic-protocol TASK-69'
  - 'sql-semantic-protocol TASK-73'
  - 'sql-semantic-protocol TASK-59'
  - 'TASK-18'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
sql-tdg currently samples inputs for GROUP BY and aggregates but cannot guarantee specific aggregate results or HAVING qualification. Add group-aware solving based solely on protocol TASK-59.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume a typed protocol group and aggregate witness contract; do not infer COUNT/SUM/HAVING semantics from SQL in this repository.
- [ ] #2 Generate deterministic row groups satisfying supported COUNT, SUM, MIN, MAX and HAVING conditions, including groups filtered out by HAVING when safely requested.
- [ ] #3 Handle duplicate contributions, NULL inputs, multiple groups, join multiplicity and unsatisfiable group conditions exactly.
- [ ] #4 Verify group totals and full output snapshots through DuckDB for both raw SQL and compiled dbt fixture models.
- [ ] #5 Expose capability restrictions and explicit errors rather than claiming all aggregates are solvable.
- [ ] #6 Generate source rows for groups/HAVING after filtered, joined and composed dbt layers, with multiple grouped aggregates, duplicate contributions and MAX/AVG cases where proven.
<!-- AC:END -->

## Protocol v3 release gate (2026-10-09)

**Release dependency:** This task must be validated against the upstream single 3.0.0 release candidate pinned by sql-tdg TASK-43; the protocol release remains blocked on upstream TASK-91. Original criteria remain binding. Do not mark Done using only an operator-local proof where the physical-source DAG cannot realize it. Fully execute SQL to verify terminal rows and deliberately rejected cases, and document supported/residual variants in the feature-by-dialect matrix. Companion planning PR: https://github.com/phdah/sql-semantic-protocol/pull/87.
