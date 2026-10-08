---
id: TASK-26
title: Generate witnesses for EXISTS and correlated subquery predicates
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'sql-semantic-protocol TASK-62'
  - 'TASK-14'
  - 'TASK-18'
priority: high
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
EXISTS/NOT EXISTS and IN/NOT IN subqueries are currently residual. Generate source datasets that meet subquery membership conditions once SQL Semantic Protocol TASK-62 describes their correlations exactly.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Consume protocol-resolved correlated and uncorrelated subquery constraints without introducing a second query analyzer.
- [ ] #2 Generate provable existence and non-existence witnesses and membership for supported IN and NOT IN forms while honoring NULL three-valued semantics.
- [ ] #3 Preserve composed predicates through CTEs and multi-layer models, and classify rejected witnesses only when nonmembership is guaranteed.
- [ ] #4 Test empty/nonempty subquery sets, nullable keys, duplicate keys and impossible correlations by executing SQL in DuckDB.
- [ ] #5 Fail closed on unsupported nested or ambiguous subquery semantics and document the supported surface.
<!-- AC:END -->
