---
id: TASK-28
title: Generate grouped aggregate and HAVING outcome witnesses
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
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
<!-- AC:END -->
