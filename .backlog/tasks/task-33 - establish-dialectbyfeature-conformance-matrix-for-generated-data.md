---
id: TASK-33
title: Establish dialect-by-feature conformance matrix for generated data
status: To Do
assignee: []
created_date: '2026-10-08'
labels: []
milestone: m-3
dependencies: []
references:
  - 'sql-semantic-protocol TASK-47'
  - 'sql-semantic-protocol TASK-51'
  - 'README.md'
priority: medium
type: feature
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Parser acceptance for thirteen dialect families is broader than generator-proven semantics. Track precise SQL feature coverage and comparison assumptions by dialect with automated tests to prevent overclaiming.

This is post-1.0 feature work, not a claim about existing CLI behavior. Canonical semantics, dialect resolution, lineage and SQL analysis belong solely to SQL Semantic Protocol. If upstream semantics are missing, implement and release the protocol prerequisite first; do not add interpretation fallbacks in sql-tdg. Preserve deterministic outputs and exactness errors.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Create a reviewed dialect-by-feature compatibility matrix for all dialects exposed by SQL Semantic Protocol, distinguishing parsing, protocol exactness, generator capability and execution verification.
- [ ] #2 Exercise representative shared syntax for filters, joins, CTEs, analytics, constraints and unsupported conditions across dialects; test documented dialect-specific variants.
- [ ] #3 Where a dialect execution engine is available, compare full results; otherwise explicitly label coverage as parse/analysis-only and avoid claiming execution correctness.
- [ ] #4 Ensure every new semantic feature in this milestone updates the matrix and its fixtures as part of definition of done.
- [ ] #5 Publish the matrix in docs and link it from README without hardcoding a production dialect whitelist.
<!-- AC:END -->
